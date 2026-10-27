use std::io;
use std::path::Path;

use crate::*;

/// **Measured** 2026-09-08 (`qa/paired/bt-audience.tsv`): the typed-input
/// prompt Blackthorn's demand ends on. Note it differs from the sage's
/// `You respond:`.
pub const BLACKTHORN_RESPONSE_PROMPT: &str = "Your response?";
/// `blackthorn.md §4.1`: "Each demand is followed by `\n\nYour response?\n:`
/// ... The colon belongs to this audience prompt. The Badge password in
/// Section 7a uses the same question words but starts input on a colon-free
/// row."
///
/// The colon is the *live* row's own prefix rather than a printed message
/// line, the same way the shrine's virtue prompt carries it: it is the row
/// the typed answer lands on, and the message window's layout places one
/// continuation row under the last printed one. Spelling it into the message
/// instead left the printed block two rows short of the window's bottom, with
/// the shell's own live row below it.
pub const BLACKTHORN_ANSWER_ROW_PREFIX: &str = ":";
pub const BLACKTHORN_INPUT_PROMPT: &str = BLACKTHORN_RESPONSE_PROMPT;
/// §4.1's second demand, record `1`. The engine had no wording for it at
/// all: the first wrong answer printed the rebuke and the threat and then
/// stopped, so the ask the player was answering was never on screen.
pub const BLACKTHORN_SECOND_DEMAND: &str = "\"Now tell me, what is the Mantra of {}?\"";
/// **Measured**: what a wrong mantra draws when the interrogation ends.
/// **Measured** 2026-09-08 (`qa/paired/bt-escalate.tsv`) with a party of
/// three, driving the loop's four asks with wrong answers. Blackthorn's
/// wording escalates and the second ask names the companion at risk.
pub const BLACKTHORN_FIRST_WRONG_LINE: &str =
    "\"Make not the mistake of laughing at me, simple one!\"";
/// `blackthorn.md §4.1`: record `8` "begins with two line feeds and the
/// quoted sand/threat prefix" and stops where the name goes. This is that
/// record's shape, for the paths that run without `MISCMSG.DAT`.
pub const BLACKTHORN_SAND_THREAT_PREFIX_FALLBACK: &str =
    "\n\n\"I will ask thee until the sand has fallen. And then will ";
/// The second and third asks.
pub const BLACKTHORN_RESISTANCE_LINE: &str = "\"Resistance is futile! Thou must yield the truth unto me! Tell me, what is the Mantra of {}?\"";
/// The fourth, shouted.
pub const BLACKTHORN_FINAL_DEMAND: &str =
    "\"My patience with thee has worn away! SPEAK UNTO ME THE MANTRA, NOW!\"";
/// The execution narration, which carries no quotes - it is narration, not
/// speech.
/// **Measured** 2026-09-08 (`qa/paired/bt-correct.tsv`): what the right
/// mantra draws. The companion dies either way; only the manner changes.
pub const BLACKTHORN_MERCIFUL_DEATH_LINE: &str = "\"I thank thee, my friend! As a token of my esteem for thine honesty, I will grant thy companion a merciful death!\"";
pub const BLACKTHORN_PENDULUM_NARRATION: &str =
    "With a wave of Blackthorn's hand, the pendulum blade falls!";
pub const BLACKTHORN_DUNGEON_THREAT: &str =
    "\"A child would catch thee in thy lies, foolish one! To the dungeon with thee!\"";

/// `blackthorn.md §4.1`'s four demands, for the paths that run without
/// `MISCMSG.DAT`. `{}` takes the interrogated shrine's virtue name.
const fn blackthorn_demand_fallback(index: usize) -> &'static str {
    match index {
        0 => "\"What is the Mantra of the Mystic Shrine of {}?\"",
        1 => BLACKTHORN_SECOND_DEMAND,
        2 => BLACKTHORN_RESISTANCE_LINE,
        _ => BLACKTHORN_FINAL_DEMAND,
    }
}

impl PlayState {
    /// `town-mode.md §10`: the fireplace id of the town burning family.
    pub const TOWN_BURNING_FIREPLACE_TILE: u8 = 0xbc;

    /// `town-mode.md §10`: the molten-lava id of the town burning family. It
    /// is the same tile the Stonegate script fills the whole live grid with,
    /// which is why that scene's survivors keep burning afterwards.
    pub const TOWN_BURNING_LAVA_TILE: u8 = 0x8f;

    /// `town-mode.md §10`: the stored line printed by the burning family.
    pub const TOWN_BURNING_MESSAGE: &'static str = "Burning!";
    /// `town-mode.md §10` poison-gas status line, stored like `Burning!`.
    pub const TOWN_POISONED_MESSAGE: &'static str = "Poisoned!";
    /// `town-mode.md` "Floor transitions": the trapdoor's stored line,
    /// terminated like `Burning!` and `Poisoned!`.
    pub const TOWN_TRAPDOOR_MESSAGE: &'static str = "A TRAPDOOR!";

    /// `blackthorn.md §4`/`§8`: "Moral standing | Durable; debited five per
    /// correct interrogation answer". The subtraction is clamped and floored
    /// at zero.
    pub const BLACKTHORN_CORRECT_ANSWER_STANDING_DEBIT: u8 = 5;

    pub fn combat_sjog_actor_direction(
        &mut self,
        actor_slot: usize,
        branch: CombatCommandBranch,
        direction: Direction,
    ) -> MoveOutcome {
        match branch {
            CombatCommandBranch::Get => self.get_combat_actor_direction(actor_slot, direction),
            CombatCommandBranch::Jimmy => self.jimmy_combat_actor_direction(actor_slot, direction),
            CombatCommandBranch::Open => self.open_combat_actor_direction(actor_slot, direction),
            CombatCommandBranch::Search => {
                self.search_combat_actor_direction(actor_slot, direction)
            }
            _ => {
                self.message = "What?".to_string();
                MoveOutcome::Blocked
            }
        }
    }

    pub fn get_combat_actor_direction(
        &mut self,
        actor_slot: usize,
        direction: Direction,
    ) -> MoveOutcome {
        let Some((actor, x, y)) =
            self.combat_sjog_target_coordinate(actor_slot, direction, GET_NOTHING_REFUSAL)
        else {
            return MoveOutcome::Blocked;
        };

        if self
            .combat_actor_slot_at(x as u8, y as u8, actor_slot)
            .is_some()
        {
            self.message = GET_NOTHING_REFUSAL.to_string();
            return MoveOutcome::Blocked;
        }
        let Some((object_slot, object)) =
            self.combat_loose_object_slot_at(x, y, actor.active_object_slot as usize)
        else {
            self.message = GET_NOTHING_REFUSAL.to_string();
            return MoveOutcome::Blocked;
        };
        self.get_combat_arena_object(object_slot, object)
    }

    /// The roster member behind an arena actor: `traps.md §2.1` has a
    /// combat-class scene bypass the acting-member prompt and act as the
    /// combatant whose turn it is.
    fn combat_chest_member(&self, actor_slot: usize) -> usize {
        self.combat_roster_slot_for_actor_slot(actor_slot)
            .unwrap_or(actor_slot)
    }

    pub fn search_combat_actor_direction(
        &mut self,
        actor_slot: usize,
        direction: Direction,
    ) -> MoveOutcome {
        let Some((actor, x, y)) =
            self.combat_sjog_target_coordinate(actor_slot, direction, SEARCH_NOTHING_FOUND)
        else {
            return MoveOutcome::Blocked;
        };

        // `containers.md §4`: "A container may be found by Search through the
        // map object table. Search announces only the container's assessed
        // trap state" - the arena's drop chest included (playtest F25).
        if self.surface_object_chest_slot_at(x, y).is_some() {
            let member = self.combat_chest_member(actor_slot);
            if let Some(outcome) = self.finish_surface_object_trap_search_at(x, y, member) {
                return outcome;
            }
        }
        if let Some((_, object)) =
            self.combat_loose_object_slot_at(x, y, actor.active_object_slot as usize)
        {
            // `combat.md §8` hands Search "to one shared world-mode delegate",
            // and the delegate's found-something wording in an arena is not
            // published; no capture exercises it either. Left silent rather
            // than borrowing the town handler's object naming.
            let _ = object;
            self.message.clear();
            return MoveOutcome::Searched;
        }

        // `combat.md §8`, the Get/Search row: the arena echoes `Search-`,
        // "then the shared cardinal-direction input" - and the shared
        // delegate prints its own result. Measured 2026-09-09
        // (`qa/paired/combat-commands.tsv`, `search` beat): the original's
        // arena shows ` Search-North`, a blank row, and
        // `Thou dost find nothing of note.` before the next turn banner.
        //
        // This replaces the 2026-09-07 note that "the arena's Get, Search and
        // X-it print no line of their own", which was read off a capture
        // taken while the engine's own echo and banner ordering were wrong.
        // It is also the frame-sequence evidence `cleak/u5-spec#231` asks for
        // to reconcile its G/S/X absence report with the original's output
        // calls.
        //
        // `commands.md §5.8` (u5-spec#298 Q4): combat's S "reaches the same
        // routine", so a furniture cell in the arena takes its location
        // phrase and the lower-case `thou dost find` row.
        self.message = format!("{}{SEARCH_NOTHING_OF_NOTE_ROW}", self.search_preamble_at(x, y));
        MoveOutcome::Searched
    }

    pub fn open_combat_actor_direction(
        &mut self,
        actor_slot: usize,
        direction: Direction,
    ) -> MoveOutcome {
        let Some((_, x, y)) =
            self.combat_sjog_target_coordinate(actor_slot, direction, "Nothing to open!")
        else {
            return MoveOutcome::Blocked;
        };
        // `combat.md §8`, the `O` row: the shared Open handler "carries no
        // combat-specific branch of its own, so in an arena it behaves as it
        // does on the surface" - a chest record at the cell runs the chest
        // helper. `§6.3`'s accepted drop is exactly such a record, and the
        // arena answered `Nothing to open!` beside one (playtest F25).
        if self.surface_object_chest_slot_at(x, y).is_some() {
            let member = self.combat_chest_member(actor_slot);
            if let Some(outcome) =
                self.consume_surface_object_chest_at(x, y, member, SurfaceChestVerb::Open.label())
            {
                return outcome;
            }
        }
        let tile = self.combat_terrain[y][x];
        if tile == TOWN_OPEN_ALREADY_OPEN_TILE {
            self.message = "It's open!".to_string();
            return MoveOutcome::DoorOpened;
        }
        if tile == TOWN_OPEN_TOO_HEAVY_TILE {
            self.message = "Too heavy!".to_string();
            return MoveOutcome::Blocked;
        }
        if jimmy_locked_door_rewrite(tile).is_some() || jimmy_magic_locked_door(tile) {
            self.message = "Locked!".to_string();
            return MoveOutcome::Blocked;
        }
        if self
            .combat_actor_slot_at(x as u8, y as u8, actor_slot)
            .is_some()
        {
            self.message = "Nothing to open!".to_string();
            return MoveOutcome::Blocked;
        }
        if !openable_town_door(tile) {
            self.message = "Nothing to open!".to_string();
            return MoveOutcome::Blocked;
        }

        self.combat_terrain[y][x] = TOWN_DOOR_CLEARED_TILE;
        self.mark_visibility_dirty();
        self.message = "Opened!".to_string();
        MoveOutcome::DoorOpened
    }

    pub fn jimmy_combat_actor_direction(
        &mut self,
        actor_slot: usize,
        direction: Direction,
    ) -> MoveOutcome {
        let Some((_, x, y)) = self.combat_sjog_target_coordinate(actor_slot, direction, "No lock!")
        else {
            return MoveOutcome::Blocked;
        };
        if self.keys == 0 {
            self.message = "No keys!".to_string();
            return MoveOutcome::Blocked;
        }
        // `combat.md §7.1` / `doors-and-z-transitions.md §3.1`: the roll uses
        // the selected member's Dexterity. A descriptor index is not a roster
        // index once a dead member has packed out, so resolve it first.
        let member_index = self.combat_chest_member(actor_slot);
        // `combat.md §6.3`: the drop chest's byte-5 high bit "is the same
        // lock/trap flag the Jimmy and Search command family reads", and
        // `containers.md §4` has a locked container "Jimmied first".
        if self.surface_object_chest_slot_at(x, y).is_some()
            && let Some(outcome) = self.jimmy_surface_object_chest_at(x, y, member_index)
        {
            return outcome;
        }
        let tile = self.combat_terrain[y][x];
        if jimmy_magic_locked_door(tile) {
            self.message = "Key broke!".to_string();
            // `audio.md §8.1`: failure only — print the break line, play
            // the 40-update action snap, then decrement the key count.
            self.emit_sound_effect(SoundEffect::ActionSnap);
            self.keys = self.keys.saturating_sub(1);
            return MoveOutcome::LockTried;
        }
        if jimmy_restraint_tile(tile) {
            if !self.jimmy_lock_pick_succeeds(member_index) {
                self.message = "Key broke!".to_string();
                // `audio.md §8.1`: failure only — print the break line,
                // play the 40-update action snap, then decrement the key
                // count.
                self.emit_sound_effect(SoundEffect::ActionSnap);
                self.keys = self.keys.saturating_sub(1);
                return MoveOutcome::LockTried;
            }
            self.combat_terrain[y][x] = TOWN_DOOR_CLEARED_TILE;
            self.mark_visibility_dirty();
            self.message = "Unlocked".to_string();
            return MoveOutcome::LockTried;
        }
        if self
            .combat_actor_slot_at(x as u8, y as u8, actor_slot)
            .is_some()
        {
            self.message = "No lock!".to_string();
            return MoveOutcome::Blocked;
        }
        let Some(unlocked_tile) = Self::visible_jimmy_unlock_tile(tile) else {
            self.message = "No lock!".to_string();
            return MoveOutcome::Blocked;
        };
        if !self.jimmy_lock_pick_succeeds(member_index) {
            self.message = "Key broke!".to_string();
            // `audio.md §8.1`: failure only — print the break line, play
            // the 40-update action snap, then decrement the key count.
            self.emit_sound_effect(SoundEffect::ActionSnap);
            self.keys = self.keys.saturating_sub(1);
            return MoveOutcome::LockTried;
        }

        self.combat_terrain[y][x] = unlocked_tile;
        self.mark_visibility_dirty();
        self.message = "Unlocked!".to_string();
        MoveOutcome::LockTried
    }

    pub fn combat_sjog_target_coordinate(
        &mut self,
        actor_slot: usize,
        direction: Direction,
        out_of_bounds_message: &str,
    ) -> Option<(CombatActorDescriptor, usize, usize)> {
        let Some(actor) = self.live_combat_party_actor(actor_slot) else {
            self.message.clear();
            return None;
        };
        if !direction.is_cardinal() {
            // The arena's prompts only accept cardinals, so a diagonal never
            // reaches a handler in play; this is the inline harness form's
            // guard and the original has no line for it.
            self.message.clear();
            return None;
        }
        let (dx, dy) = direction.delta();
        let x = actor.x as isize + dx;
        let y = actor.y as isize + dy;
        if !combat_arena_coordinate_in_bounds(x as i16, y as i16) {
            self.message = out_of_bounds_message.to_string();
            return None;
        }
        Some((actor, x as usize, y as usize))
    }

    pub fn klimb_combat_actor_vertical(
        &mut self,
        actor_slot: usize,
        intent: ClimbIntent,
    ) -> MoveOutcome {
        let Some(actor) = self.live_combat_party_actor(actor_slot) else {
            self.message.clear();
            return MoveOutcome::Blocked;
        };
        let tile = self.combat_terrain[actor.y as usize][actor.x as usize];
        if let Some((encounter_mode_high_bit, two_way_centre)) =
            self.combat_dungeon_arena_klimb_context(&actor)
        {
            return self.klimb_dungeon_arena_departure(
                actor_slot,
                tile,
                intent,
                encounter_mode_high_bit,
                two_way_centre,
            );
        }
        // u5-spec#299 Q4 (`combat.md §8.4`): the ladder cases test no arena
        // kind, so a town or overworld arena's `0xC8`/`0xC9` departs the same
        // way. What those callers then do with exit code 5 or 6 is not
        // established (`OPEN-QUESTIONS.md`); no stock arena can produce one.
        if matches!(tile, TOWN_KLIMB_ASCEND_TILE | TOWN_KLIMB_DESCEND_TILE) {
            return self.klimb_dungeon_arena_departure(actor_slot, tile, intent, false, false);
        }
        if !combat_klimb_tile_accepts_vertical(tile, intent) {
            self.message = DUNGEON_KLIMB_WHAT_REFUSAL.trim_end().to_string();
            return MoveOutcome::Blocked;
        }

        let label = match intent {
            ClimbIntent::Up => "up",
            ClimbIntent::Down => "down",
        };
        // Unpublished (`cleak/u5-spec#262`), like the town/overworld Klimb.
        self.diagnostics
            .push(format!("klimbed {label} from combat"));
        self.message.clear();
        // Successful vertical Klimb restores the suspended frame immediately,
        // so its committed-action maintenance must run while the acting
        // descriptor is still present.
        let _ = self.apply_combat_absorbable_field_contact_for_actor_position(actor_slot);
        let _ = self.apply_combat_post_dispatch_contact_for_actor_position(actor_slot);
        let _ = self.apply_visible_combat_magic_ring_pass_to_slot(actor_slot);
        let _ = self.age_active_effect();
        let exit = match self.combat_round_loop_control(true, false) {
            CombatRoundLoopControl::Exit(exit) => exit,
            CombatRoundLoopControl::ContinueActorWalk | CombatRoundLoopControl::StartNextRound => {
                CombatRoundLoopExit::LeaveCombat
            }
        };
        self.apply_combat_round_loop_exit(exit);
        MoveOutcome::Moved
    }

    /// `dungeon-mode.md §14.1`: in a dungeon arena, whether the arena is a
    /// dungeon-room encounter, and whether `actor` stands on the ambush
    /// centre the painter built on a two-way-ladder dungeon cell (the
    /// two-way flag it raises). `None` outside a dungeon arena.
    fn combat_dungeon_arena_klimb_context(
        &self,
        actor: &CombatActorDescriptor,
    ) -> Option<(bool, bool)> {
        self.combat_frame_snapshot
            .as_ref()
            .filter(|snapshot| matches!(snapshot.area, Area::Dungeon { .. }))
            .map(|snapshot| {
                (
                    snapshot.encounter_mode_high_bit,
                    self.tile_restoration_pending
                        && !snapshot.encounter_mode_high_bit
                        && (actor.x, actor.y) == (5, 5),
                )
            })
    }

    /// `combat.md §8` (u5-spec#298 Q1): "K prints `Klimb-` and resolves its
    /// current-cell ladder cases first. Up/Down ladder results are `Up!\n` /
    /// `Down!\n`; a dual ladder adds `U/D-` before its choice", and "the
    /// dual-ladder `U/D-` form appears only on the ambush arena's centre cell
    /// when the dungeon cell was a two-way ladder". Only a cell that is not
    /// one of these cases reaches "the ordinary adjacent-climb branch", which
    /// waits for the cardinal input.
    ///
    /// u5-spec#299 Q4 (`combat.md §8.4`, "Current-cell cases, in order, in
    /// every arena kind"): the two-way `U/D-` form needs the dungeon
    /// encounter mode and the two-way-ladder flag; "`0xC8` gives `Up!` and
    /// `0xC9` gives `Down!`. Neither case tests the arena kind or the
    /// scene"; `0x86` counts as `Down!` only in a dungeon-room encounter;
    /// otherwise K goes to the adjacent-climb prompt. No stock town or
    /// overworld arena has a `0xC8`/`0xC9` cell.
    pub fn combat_klimb_current_cell(&self, actor_slot: usize) -> Option<CombatKlimbCurrentCell> {
        let actor = self.live_combat_party_actor(actor_slot)?;
        let (room_encounter, two_way_centre) = self
            .combat_dungeon_arena_klimb_context(&actor)
            .unwrap_or((false, false));
        let tile = *self
            .combat_terrain
            .get(actor.y as usize)?
            .get(actor.x as usize)?;
        match tile {
            TOWN_KLIMB_ASCEND_TILE if two_way_centre => Some(CombatKlimbCurrentCell::Both),
            TOWN_KLIMB_ASCEND_TILE => Some(CombatKlimbCurrentCell::Up),
            TOWN_KLIMB_DESCEND_TILE => Some(CombatKlimbCurrentCell::Down),
            TOWN_KLIMB_DESCEND_GRATE_TILE if room_encounter => Some(CombatKlimbCurrentCell::Down),
            _ => None,
        }
    }

    /// `combat.md §8` K row and `dungeon-mode.md §14.1` (u5-spec#298 Q1): in
    /// a dungeon arena `Up!` on an up ladder (`0xC8`) departs with exit code
    /// 5 and `Down!` on a down ladder (`0xC9`), or on a metal grate (`0x86`)
    /// in a dungeon-room encounter, with code 6; on the ambush centre built on
    /// a two-way ladder, down counts as the `0xC9` case. The code is stored
    /// in the first-party-exit record only if it is still empty; in a room
    /// encounter a later party departure must match it or is refused with
    /// `All must use the same exit!`. The climb word is followed by the
    /// ordinary `Leave!` or `Escape!` departure line, and the departure
    /// releases only this actor, as an edge departure does (`combat.md §3`).
    fn klimb_dungeon_arena_departure(
        &mut self,
        actor_slot: usize,
        tile: u8,
        intent: ClimbIntent,
        room_encounter: bool,
        two_way_centre: bool,
    ) -> MoveOutcome {
        let code = match intent {
            ClimbIntent::Up if tile == TOWN_KLIMB_ASCEND_TILE => 5,
            ClimbIntent::Down if tile == TOWN_KLIMB_DESCEND_TILE => 6,
            ClimbIntent::Down if tile == TOWN_KLIMB_DESCEND_GRATE_TILE && room_encounter => 6,
            ClimbIntent::Down if tile == TOWN_KLIMB_ASCEND_TILE && two_way_centre => 6,
            _ => {
                self.message = DUNGEON_KLIMB_WHAT_REFUSAL.trim_end().to_string();
                return MoveOutcome::Blocked;
            }
        };
        let word = if code == 5 {
            DUNGEON_KLIMB_UP
        } else {
            DUNGEON_KLIMB_DOWN
        };
        let party_side = self
            .combat_actors
            .get(actor_slot)
            .is_some_and(|actor| actor.is_party_side());
        let record = self
            .combat_frame_snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.established_exit_direction_code);
        // The word completes whichever form K left open: `Klimb-` on a
        // single ladder, `Klimb-U/D-` on the two-way ambush centre.
        if !(two_way_centre
            && self.complete_open_direction_echo(DUNGEON_KLIMB_PROMPT_BOTH, word.trim_end()))
            && !self.complete_open_direction_echo(DUNGEON_KLIMB_PROMPT_ONE, word.trim_end())
        {
            self.emit_message_line(word);
        }
        self.message.clear();
        if party_side && room_encounter && record.is_some_and(|stored| stored != code) {
            // `combat.md §8.4` "Turn cost" (u5-spec#299 Q13): the shared-exit
            // refusal "prints the ladder line, a blank row, then `All must use
            // the same exit!` with its newline, and plays the 165 Hz /
            // 200-unit bump tone", as the edge helper does for a direction
            // key; the refusal is free (the caller re-prompts on `Blocked`).
            self.emit_message_line(format!("\n{COMBAT_ALL_MUST_USE_SAME_EXIT_LINE}"));
            self.emit_sound_effect(SoundEffect::BlockedStep);
            return MoveOutcome::Blocked;
        }
        if party_side && record.is_none() {
            if let Some(snapshot) = &mut self.combat_frame_snapshot {
                snapshot.established_exit_direction_code = Some(code);
            }
        }
        let departure = if combat_has_active_not_dead_non_party_actor(&self.combat_actors) {
            COMBAT_DEPARTURE_ESCAPE_LINE
        } else {
            COMBAT_DEPARTURE_LEAVE_LINE
        };
        self.emit_message_line(departure);
        self.active_player = None;
        if let Some(actor) = self.combat_actors.get_mut(actor_slot) {
            let active_object_slot = usize::from(actor.active_object_slot);
            if !actor.is_empty() {
                actor.release_preserving_owner_target_class();
            }
            if let Some(object) = self.active_objects.get_mut(active_object_slot)
                && !object.is_empty()
            {
                object.clear_record_prefix();
            }
        }
        self.advance_visual_tick();
        self.mark_visibility_dirty();
        self.emit_sound_effect(SoundEffect::ActionSnap);
        MoveOutcome::Moved
    }

    pub fn klimb_combat_actor_direction(
        &mut self,
        actor_slot: usize,
        direction: Direction,
    ) -> MoveOutcome {
        let Some(actor) = self.live_combat_party_actor(actor_slot) else {
            self.message.clear();
            return MoveOutcome::Blocked;
        };
        if !direction.is_cardinal() {
            self.message = crate::commands::COMBAT_KLIMB_ADJACENT_REFUSAL.to_string();
            return MoveOutcome::Blocked;
        }

        let (dx, dy) = direction.delta();
        let x = actor.x as isize + dx;
        let y = actor.y as isize + dy;
        if !combat_arena_coordinate_in_bounds(x as i16, y as i16) {
            self.message = crate::commands::COMBAT_KLIMB_ADJACENT_REFUSAL.to_string();
            return MoveOutcome::Blocked;
        }
        let x = x as usize;
        let y = y as usize;
        // `combat.md §8.4` "Adjacent-climb prompt" (u5-spec#299 Q4, Q13): "The
        // only climbable adjacent target is a pile of rocks (`0x4C`) with no
        // object on it. The town command's wooden-fence targets
        // (`0xCA`/`0xCB`) are not climbable in an arena. A direction toward
        // any other cell prints `What?\n` and re-prompts at no cost." Any
        // object on the rocks - an actor's or a loose chest or drop - refuses
        // the same way. This used to climb onto any walkable cell (playtest
        // E39).
        let actor_object_slot = usize::from(actor.active_object_slot);
        let object_on_target = self
            .active_objects
            .iter()
            .enumerate()
            .any(|(slot, object)| {
                slot != actor_object_slot && !object.is_empty() && object.x == x && object.y == y
            });
        if self.combat_terrain[y][x] != COMBAT_GARGOYLE_DEATH_TERRAIN_TILE
            || object_on_target
            || self
                .combat_actor_slot_at(x as u8, y as u8, actor_slot)
                .is_some()
        {
            self.message = crate::commands::COMBAT_KLIMB_ADJACENT_REFUSAL.to_string();
            return MoveOutcome::Blocked;
        }

        let Some(commit) = commit_combat_actor_linked_position(
            &mut self.combat_actors[actor_slot],
            &mut self.active_objects,
            x as u8,
            y as u8,
        ) else {
            self.message.clear();
            return MoveOutcome::Blocked;
        };
        self.mark_visibility_dirty();
        // `combat.md §8.4`: the applied climb "also runs the ordinary step-on
        // ambush/camp reveal check (Section 5, \"Ambush and camp reveal
        // slots\"), which prints nothing but may redraw the arena".
        let _ = self.apply_combat_ambush_reveal_for_actor_position(actor_slot);
        // `combat.md §8.4`: an applied climb "prints nothing after the
        // direction name".
        self.diagnostics.push(format!(
            "klimbed {} to ({}, {})",
            direction.name(),
            commit.actor_position_after.0,
            commit.actor_position_after.1
        ));
        self.message.clear();
        MoveOutcome::Moved
    }

    fn live_combat_party_actor(&self, actor_slot: usize) -> Option<CombatActorDescriptor> {
        if !self.combat_active || actor_slot >= COMBAT_PARTY_ACTOR_SLOTS {
            return None;
        }
        self.combat_actors
            .get(actor_slot)
            .copied()
            .filter(|actor| combat_actor_is_active_not_dead(*actor))
    }

    pub fn open_dungeon_chest(
        &mut self,
        scene: DungeonScene,
        level: u8,
        x: usize,
        y: usize,
        idx: usize,
        tile: u8,
    ) -> MoveOutcome {
        // `traps.md §2.1`/§4: the dungeon chest site uses the same shared
        // acting-member selection, minus the combat override - the Open
        // dispatcher never routes a combat-class scene here. The selection
        // runs before the trap test, so a party with nobody able aborts
        // before the trap can fire.
        let acting_member = self.dungeon_container_acting_member();
        let target_slot = match acting_member {
            ActingMemberSelection::Selected(slot) => slot,
            ActingMemberSelection::Prompt => {
                self.active_surface_chest = Some(SurfaceChestSession {
                    highlight: self.acting_member_picker_initial_highlight(),
                    ..SurfaceChestSession::new_dungeon(scene, level, x, y, idx, tile)
                });
                self.message = self.render_active_surface_chest();
                return MoveOutcome::Observed;
            }
            ActingMemberSelection::NoneAble => {
                // `dungeon-mode.md §8.1`, "Who acts": Open runs the shared
                // acting-member prompt "before anything else", and "a party
                // with no eligible member at all, answers `None!\n` and the
                // command aborts" - on any cell, now that the cell is read
                // only after the pick (playtest E12).
                self.message = DUNGEON_ACTING_MEMBER_NONE.to_string();
                return MoveOutcome::Blocked;
            }
        };
        self.finish_open_dungeon_chest(scene, level, x, y, idx, tile, target_slot)
    }

    pub(crate) fn finish_open_dungeon_chest(
        &mut self,
        scene: DungeonScene,
        level: u8,
        x: usize,
        y: usize,
        idx: usize,
        tile: u8,
        target_slot: usize,
    ) -> MoveOutcome {
        if self.grid.get(idx).copied() != Some(tile) || tile >> 4 != 0x4 {
            // `dungeon-mode.md §8.1` chest-command table: "already-open
            // cell: `Already Open!\n` - capital `O`, a **different** literal
            // from Jimmy's; anything else: `What?\n`. Because those last two
            // carry no leading line feed they render on the prefix's own
            // row, as `Open-Already Open!` and `Open-What?`"
            if self.grid.get(idx).copied() == Some(tile) && tile >> 4 == 0x7 {
                self.advance_turn();
                self.message = DUNGEON_CHEST_OPEN_ALREADY_OPEN.to_string();
                return MoveOutcome::ContainerOpened;
            }
            self.message = DUNGEON_CHEST_OPEN_WHAT.to_string();
            return MoveOutcome::Blocked;
        }
        // `dungeon-mode.md §8.1`, "The dungeon chest lifecycle, and what
        // O-Open does and does not consult" (established 2026-09-12, issue
        // #262): "**The trap condition is the cell's low three bits**, its
        // lock/trap sub-type - not the low bit alone." There is no locked
        // outcome and no lock-difficulty read; the detection roll this used to
        // consult belongs to Search, not to Open.
        // The dispatcher's `Open-` echo leaves its row open for the result,
        // and `emit_command_echo_line` does not record that, so a flag left
        // closed by the turn before (`Must open first!\n`) put the trap word
        // on a row of its own and an untrapped chest's leading feed spent a
        // blank under the echo. Measured 2026-10-16 (`dungeon-chest`, beat
        // `opened`: stock `>Open-GAS!` / blank / `Chest opened`, this engine
        // `>Open-` / `ACID!`; `dungeon-chest-jimmy`, beats `opened` and
        // `again`: stock `>Open-` directly above `Chest opened`).
        if self
            .message_transcript
            .last()
            .is_some_and(|entry| entry.text == DUNGEON_CHEST_OPEN_ECHO)
        {
            self.open_message_row();
        }
        let trap_note = if tile & DUNGEON_CHEST_TRAP_SUBTYPE_MASK != 0 {
            // The resolver prints its own word; "The dungeon chest site prints
            // no trap notice of its own."
            // Only when the row the word would join is the dispatcher's
            // own `Open-` echo. A chest opened through the acting-member
            // picker has that member's name on the last row instead, and
            // the word must not complete *that* - it is the row the
            // picker's echo owns.
            let joins_open_echo = self
                .message_transcript
                .last()
                .is_some_and(|entry| entry.text == DUNGEON_CHEST_OPEN_ECHO);
            Some(if joins_open_echo {
                self.apply_shared_trap_effect_to_slot_continuing_row(target_slot)
            } else {
                self.apply_shared_trap_effect_to_slot(target_slot)
            })
        } else {
            None
        };
        self.grid[idx] = dungeon_open_chest_rewrite(tile);
        self.mark_visibility_dirty();
        self.advance_turn();
        self.diagnostics.push(match trap_note {
            Some(trap) => format!(
                "Opened dungeon chest at ({x}, {y}) on {} level {level}; {trap}, marked visit-local open chest.",
                scene.key()
            ),
            None => format!(
                "Opened dungeon chest at ({x}, {y}) on {} level {level}; marked visit-local open chest.",
                scene.key()
            ),
        });
        // "**The cell rewrite and the opened line happen identically whether
        // or not a trap fired.** ... then `\nChest opened\n` prints."
        //
        // Emitted rather than appended: `emit_message_line` leaves the word
        // it printed in the compatibility slot as well as the transcript, so
        // appending to that slot printed the trap word a second time.
        // Measured 2026-09-19 (`qa/paired/dungeon-chest.tsv`, beat
        // `opened`), where this engine read `ACID!` / `ACID!`.
        self.emit_message_line(DUNGEON_CHEST_OPENED);
        MoveOutcome::ContainerOpened
    }

    /// `dungeon-mode.md §8` chest Search and §8.1 "Search outcomes": the
    /// detection roll's tier selects exactly one of the four published
    /// trap-tier lines, "**none of which carries a terminal period**". They
    /// are the one outcome line that follows the unconditional `You find:`
    /// preamble, so this arm prints no narration of its own.
    pub fn search_dungeon_chest(
        &mut self,
        _scene: DungeonScene,
        level: u8,
        x: usize,
        y: usize,
        tile: u8,
        actor: Option<usize>,
    ) -> MoveOutcome {
        let detail = self.dungeon_chest_trap_detail(level, x, y, tile, actor);
        self.advance_turn();
        self.message = dungeon_chest_search_trap_line(detail).to_string();
        MoveOutcome::Searched
    }

    /// `dungeon-mode.md` Section 8, chest Search: the threshold is
    /// `(2 * Z - member Dexterity + 30) / 2` for the acting member, and both
    /// the `1..30` roll and the fresh `1..8` tier roll are shared-PRNG draws.
    pub fn dungeon_chest_trap_detail(
        &mut self,
        level: u8,
        x: usize,
        y: usize,
        tile: u8,
        actor: Option<usize>,
    ) -> &'static str {
        let _ = (x, y);
        let dexterity = self.dungeon_search_actor_dexterity(actor);
        let threshold = Self::dungeon_chest_pick_threshold(level, dexterity);
        let roll = self.random_range_u8(1, 30);
        let detail = if u16::from(roll) > threshold && Self::is_plain_closed_dungeon_chest(tile) {
            "no trap"
        } else {
            let tier = if u16::from(roll) <= threshold {
                self.random_range_u8(1, 8)
            } else {
                level
            };
            match tier {
                0..=3 => "simple trap",
                7..=u8::MAX => "complex trap",
                _ => "trap",
            }
        };
        detail
    }

    /// `dungeon-mode.md` Section 8.1, "Who acts": the Dexterity of the member
    /// the shared acting-member prompt chose. Without an explicit pick this
    /// is the member the prompt selects silently, else the first eligible
    /// (Good or Poisoned) member.
    pub fn dungeon_search_actor_dexterity(&self, actor: Option<usize>) -> u8 {
        let index = actor.or_else(|| match self.acting_member_selection() {
            ActingMemberSelection::Selected(slot) => Some(slot),
            _ => self
                .party
                .iter()
                .position(|member| acting_member_status_eligible(member.status)),
        });
        index
            .and_then(|index| self.party.get(index))
            .map(|member| member.dexterity())
            .unwrap_or_default()
    }

    pub fn dungeon_chest_pick_threshold(level: u8, dexterity: u8) -> u16 {
        dungeon_chest_jimmy_threshold(level, dexterity)
    }

    pub fn is_plain_closed_dungeon_chest(tile: u8) -> bool {
        tile == 0x40
    }

    pub fn consume_dungeon_chest(
        &mut self,
        entries: Option<&[DungeonChestContentEntry]>,
        scene: DungeonScene,
        level: u8,
        x: usize,
        y: usize,
        idx: usize,
        tile: u8,
        verb: &str,
    ) -> MoveOutcome {
        // `containers.md §6`: "**The generator is the whole contract. There is
        // no authored per-chest override.** ... An implementation carrying an
        // authored per-chest content sidecar has no counterpart for it in the
        // original and should retire it."
        let _ = entries;
        let rows = self.generate_dungeon_chest_content(level);
        self.consume_dungeon_chest_with_note(scene, level, x, y, idx, tile, verb, &rows)
    }

    /// `containers.md §6` dungeon-chest reward generator. Iterates the
    /// seven published reward rows in order, driven by
    /// [`DUNGEON_CHEST_ROWS`] rather than by re-typed literals, so the
    /// gate thresholds and the row order have one source of truth.
    /// Each row rolls uniform in `1..=(4 * dungeon_depth + 4)`
    /// ([`dungeon_chest_row_gate_max`]) and is awarded when its
    /// threshold is at or below that roll
    /// ([`dungeon_chest_row_awarded`]); multiple rows can succeed for
    /// one chest.
    ///
    /// "The generator's complete input set is: the party's dungeon depth;
    /// the party's X and Y **used only to index the underfoot cell**; the
    /// cell byte itself, which is masked so that its low nibble is
    /// discarded ...; three fixed seven-entry tables ...; and the shared
    /// uniform random helper." So every draw comes from the shared
    /// gameplay PRNG and nothing per-coordinate is an input here.
    pub fn generate_dungeon_chest_content(&mut self, level: u8) -> String {
        let gate_upper = dungeon_chest_row_gate_max(level);
        let mut parts = Vec::new();

        for row in DUNGEON_CHEST_ROWS.iter() {
            let gate_roll = self.random_range_u8(1, gate_upper);
            if !dungeon_chest_row_awarded(*row, gate_roll) {
                continue;
            }
            match row.reward {
                DungeonChestReward::Food => {
                    let amount = self.random_range_u8(1, DUNGEON_CHEST_FOOD_MAX);
                    self.apply_object_pickup(ObjectPickupKind::Food, amount);
                    parts.push(ObjectPickupKind::Food.pickup_result_line(amount));
                }
                DungeonChestReward::Gold => {
                    let amount = self.dungeon_chest_gold_roll(level);
                    self.apply_object_pickup(ObjectPickupKind::Gold, amount);
                    parts.push(ObjectPickupKind::Gold.pickup_result_line(amount));
                }
                DungeonChestReward::Keys => {
                    let amount = self.random_range_u8(1, DUNGEON_CHEST_SMALL_MAX);
                    self.apply_object_pickup(ObjectPickupKind::Keys, amount);
                    parts.push(ObjectPickupKind::Keys.pickup_result_line(amount));
                }
                DungeonChestReward::Gems => {
                    let amount = self.random_range_u8(1, DUNGEON_CHEST_SMALL_MAX);
                    self.apply_object_pickup(ObjectPickupKind::Gems, amount);
                    parts.push(ObjectPickupKind::Gems.pickup_result_line(amount));
                }
                DungeonChestReward::Torches => {
                    let amount = self.random_range_u8(1, DUNGEON_CHEST_SMALL_MAX);
                    self.apply_object_pickup(ObjectPickupKind::Torches, amount);
                    parts.push(ObjectPickupKind::Torches.pickup_result_line(amount));
                }
                DungeonChestReward::Potion => {
                    let subtype = usize::from(self.random_range_u8(0, DUNGEON_CHEST_SUBTYPE_MAX));
                    self.apply_object_pickup(ObjectPickupKind::Potion(subtype), 1);
                    parts.push(ObjectPickupKind::Potion(subtype).pickup_result_line(1));
                }
                DungeonChestReward::Scroll => {
                    let subtype = usize::from(self.random_range_u8(0, DUNGEON_CHEST_SUBTYPE_MAX));
                    self.apply_object_pickup(ObjectPickupKind::Scroll(subtype), 1);
                    parts.push(ObjectPickupKind::Scroll(subtype).pickup_result_line(1));
                }
            }
        }

        // `containers.md §6` "What each row prints": "one row per row that
        // fires, in the table's order". "**A chest whose rows all fail prints
        // the preamble and nothing else.** There is no "empty" line on this
        // path."
        self.diagnostics.push(if parts.is_empty() {
            "generated chest grants nothing".to_string()
        } else {
            format!("generated chest grants {}", parts.join(", "))
        });
        parts.join("\n")
    }

    /// `containers.md §6` gold row. The row passes lower endpoint `1`
    /// and upper endpoint `8 * dungeon_depth`
    /// ([`dungeon_chest_gold_upper`]) to the shared one-based random
    /// helper. At `dungeon_depth == 0` that is an invalid `1..0`
    /// range; the published behaviour is to consume one PRNG advance
    /// and reach the zero-width edge rather than clamp the bound to
    /// one, so [`dungeon_chest_gold_is_zero_width`] gates that branch.
    pub fn dungeon_chest_gold_roll(&mut self, level: u8) -> u8 {
        if dungeon_chest_gold_is_zero_width(level) {
            self.prng_state = u5_prng_advance_state(self.prng_state);
            return 0;
        }
        self.random_range_u8(1, dungeon_chest_gold_upper(level))
    }

    pub fn consume_dungeon_chest_with_note(
        &mut self,
        scene: DungeonScene,
        level: u8,
        x: usize,
        y: usize,
        idx: usize,
        tile: u8,
        verb: &str,
        note: &str,
    ) -> MoveOutcome {
        self.grid[idx] = tile & 0x08;
        self.mark_visibility_dirty();
        self.advance_turn();
        self.diagnostics.push(format!(
            "{verb} dungeon chest at ({}, {}) on {} level {level}, \
             marked visit-local passage",
            x,
            y,
            scene.key()
        ));
        // `containers.md §6`: the open-chest arm prints the three-row preamble
        // `contents` / `of chest` / `You find:` under the handler's own `Get`
        // row, then one row per reward row that fired.
        self.message.push_str(DUNGEON_CHEST_GET_CONTENTS);
        self.message.push_str(note);
        MoveOutcome::ContainerOpened
    }

    /// `overworld.md §8.1` step 1: the whirlpool swallow banner, with its
    /// leading line feed.
    ///
    /// u5-spec#299 Q8 (`text-output.md §10.2`): after a hoisted step that
    /// echoed nothing the cursor waits at column 25 of the bare marker row,
    /// and "a literal that begins with a line feed, such as ... `WHIRLPOOL!`,
    /// moves to the row below and leaves the marker row holding only the
    /// marker" - the feed ends the marker row rather than leaving a blank one
    /// under it.
    pub(crate) fn emit_overworld_whirlpool_banner(&mut self) {
        self.flush_message_slot();
        if self
            .message_transcript
            .last()
            .is_some_and(|entry| entry.bare_marker)
        {
            let banner = OVERWORLD_WHIRLPOOL_BANNER.trim_start_matches('\n');
            self.emit_message_line(banner);
            let row = banner.trim_end_matches('\n');
            if let Some(entry) = self
                .message_transcript
                .iter_mut()
                .rev()
                .find(|entry| entry.text == row)
            {
                entry.starts_fresh_row = true;
            }
        } else {
            self.emit_message_line(OVERWORLD_WHIRLPOOL_BANNER);
        }
    }

    pub fn board_vehicle(&mut self) -> MoveOutcome {
        // `vehicles.md §4.1` (issue #299 Q25): every outcome is the text the
        // handler adds after the dispatcher's `Board_` prefix. The slot holds
        // it without the closing line feed, as X-it's do.
        if matches!(self.area, Area::Dungeon { .. }) {
            // "Dungeon scene | `\nNot here!\n` | `Board`, then `Not here!`".
            self.message = BOARD_NOT_HERE_REFUSAL.to_string();
            self.message_row_open_mid_line = false;
            return MoveOutcome::Blocked;
        }

        let Some(candidate) = self.boardable_vehicle_slot() else {
            // "Nothing boardable under the party | `What?\n` | `Board What?`".
            self.message = "What?".to_string();
            self.message_row_open_mid_line = false;
            return MoveOutcome::Blocked;
        };
        let transport = candidate.transport;
        let starting_transport_marker = self.player.transport.save_marker();
        let boarding_from_skiff = matches!(self.player.transport, TransportState::Skiff { .. });
        // `vehicles.md §4`, Horse: the availability check "is made
        // **before** the on-foot check, so a party in such a scene that is
        // not on foot and tries to board an occupied horse gets `"Nay!"`,
        // not `On foot`" (issue #299 Q25); §4.1 prints it double quotes
        // included.
        if matches!(self.area, Area::Town { .. })
            && transport.is_horse()
            && candidate.blocked_by_occupant
        {
            self.message = BOARD_HORSE_NAY_REFUSAL.to_string();
            self.message_row_open_mid_line = false;
            return MoveOutcome::Blocked;
        }
        if !self.player.transport.can_board(transport) {
            // `vehicles.md §4.1`: "`\nOn foot\n` | `Board`, then `On foot`,
            // with no `!`" - Board's own refusal, not the `On_foot!\n` the
            // Attack-family and dungeon-entry refusals share
            // (`commands.md §5.5`).
            self.message = BOARD_ON_FOOT_REFUSAL.to_string();
            self.message_row_open_mid_line = false;
            return MoveOutcome::Blocked;
        }

        // `stats-panel.md §2.2.1` row 11: Board's vehicle marker, hull and
        // skiff/carpet-count changes are deferred.
        self.stats_panel_command_refresh = crate::stats_panel::PanelRefreshMode::Deferred;
        self.free_active_object_slot(candidate.slot);
        // `vehicles.md §4.1`: "The order on a ship boarding is fixed: the
        // ship line, the hull test, the carpet stow when it applies, the
        // skiff-count adjustment, then the skiff test."
        let mut message = transport.board_line().to_string();
        let mut transport = transport;
        if let TransportState::Ship { hull, .. } = transport
            && hull < 10
        {
            message.push('\n');
            message.push('\n');
            message.push_str(SHIP_BADLY_DAMAGED_WARNING);
        }
        if matches!(transport, TransportState::Ship { .. })
            && ship_boarding_stows_carpet(starting_transport_marker)
        {
            let slot = &mut self.special_items[SPECIAL_ITEM_MAGIC_CARPET_INDEX];
            *slot = slot.saturating_add(1).min(PARTY_BYTE_STOCK_CAP);
        }
        if let TransportState::Ship { skiffs, .. } = &mut transport {
            // `vehicles.md §4` Ship (issue #299 Q25, `RETRACTIONS.md` R581):
            // the carried-skiff count is "the ship object's byte `+7`, **plus
            // one when the party boards from a skiff** (that skiff is taken
            // aboard)", and the warning tests that resulting count.
            if boarding_from_skiff {
                *skiffs = skiffs.saturating_add(1);
            }
            if *skiffs == 0 {
                message.push('\n');
                message.push('\n');
                message.push_str(SHIP_NO_SKIFFS_WARNING);
            }
        }
        self.player.transport = transport;
        self.sync_player_object();
        self.mark_visibility_dirty();
        self.advance_turn();
        self.message = message;
        self.message_row_open_mid_line = false;
        MoveOutcome::Boarded
    }

    pub fn force_foot_transport(&mut self) {
        self.player.transport = TransportState::Foot;
        self.sail_cadence = 0;
        self.sail_cached_direction = None;
    }

    pub fn free_active_object_slot(&mut self, slot: usize) {
        if slot == 0 {
            return;
        }
        let dungeon_monster =
            matches!(self.area, Area::Dungeon { .. }) && slot == DUNGEON_ACTIVE_MONSTER_SLOT;
        let in_arena = self.combat_active;
        if let Some(object) = self.active_objects.get_mut(slot) {
            object.free();
            if dungeon_monster {
                // `dungeon-mode.md §4.1` (u5-spec#300 Q24): the inactive form
                // is type and tile zero, class `0xFF` **and X/Y 255**, because
                // "A-Attack does not test the inactive marker at all; it
                // relies on those off-grid coordinates never matching a
                // cell". A freed record left on its old cell would match a
                // later A-Attack there now that A-Attack ignores the class.
                // (An arena's slot 1 is a combat object, not this record.)
                object.tile = 0;
                if !in_arena {
                    object.x = DUNGEON_ACTIVE_OBJECT_INACTIVE_COORDINATE;
                    object.y = DUNGEON_ACTIVE_OBJECT_INACTIVE_COORDINATE;
                }
                object.aux1 = DUNGEON_MONSTER_INACTIVE_DEP1;
            }
        }
    }

    pub fn clear_consumed_active_object_slot(&mut self, slot: usize) {
        if slot == 0 {
            return;
        }
        if let Some(object) = self.active_objects.get_mut(slot) {
            object.clear_record_prefix();
        }
    }

    pub fn clear_non_player_active_objects(&mut self) {
        self.sync_player_object();
        for object in self.active_objects.iter_mut().skip(1) {
            *object = ActiveObject::empty();
        }
    }

    pub fn clear_moonstone_pickups(&mut self, slot_index: usize) -> bool {
        let mut removed = false;
        for object in self.active_objects.iter_mut().skip(1) {
            if object.moonstone_slot_index() == Some(slot_index) {
                object.free();
                removed = true;
            }
        }
        removed
    }

    pub fn moonstone_pickup_exists(&self, slot_index: usize) -> bool {
        self.active_objects
            .iter()
            .any(|object| object.moonstone_slot_index() == Some(slot_index))
    }

    pub fn moonstone_pickup_at(&self, x: usize, y: usize) -> Option<(usize, usize)> {
        self.active_objects
            .iter()
            .enumerate()
            .skip(1)
            .find_map(|(object_slot, object)| {
                if self.object_occupies(*object, x, y) {
                    object
                        .moonstone_slot_index()
                        .map(|slot_index| (object_slot, slot_index))
                } else {
                    None
                }
            })
    }

    pub fn get_moonstone_pickup_at(&mut self, x: usize, y: usize) -> Option<MoveOutcome> {
        let (object_slot, slot_index) = self.moonstone_pickup_at(x, y)?;
        self.clear_consumed_active_object_slot(object_slot);
        self.moonstone_slots[slot_index] = MoonstoneGateSlot::invalid();
        self.mark_visibility_dirty();
        self.advance_turn();
        self.diagnostics.push(format!(
            "recovered Moonstone phase {}; Gate Travel slot cleared",
            slot_index + 1
        ));
        // `commands.md §5.8`: "The Get that follows prints `A_moonstone!\n`
        // and records the stone as carried" - the pickup table's named
        // Moonstone row, after the leading feed (playtest E60).
        self.message = format!(
            "\n{}",
            crate::world_tables::ObjectPickupKind::Moonstone(slot_index).pickup_result_line(1)
        );
        Some(MoveOutcome::Got)
    }

    pub fn allocate_active_object_slot(&mut self, object: ActiveObject) -> Option<usize> {
        if self.active_objects.is_empty() {
            return None;
        }
        // `encounters.md §9` / `active-objects.md §4`: the ordinary acquisition
        // path searches only slots 1..=23. Slot 0 is the player and slots
        // 24..=31 are reserved for setup paths outside the allocator, so
        // handing those out here over-fills the table past the published
        // density cap.
        if let Some(slot) = self
            .active_objects
            .iter()
            .enumerate()
            .skip(1)
            .take(ACTIVE_OBJECT_ACQUISITION_LAST_SLOT)
            .find_map(|(slot, object)| object.is_empty().then_some(slot))
        {
            self.active_objects[slot] = object;
            return Some(slot);
        }
        if self.active_objects.len() <= ACTIVE_OBJECT_ACQUISITION_LAST_SLOT {
            self.active_objects.push(object);
            return Some(self.active_objects.len() - 1);
        }
        // `active-objects.md §4`: taking an empty ordinary slot "is not the
        // whole contract: if the ordinary range is full, acquisition can evict
        // a lower-priority object." Run the deterministic ten-phase cascade
        // rather than failing the acquisition.
        let slot = self.active_object_eviction_victim()?;
        self.free_active_object_slot(slot);
        self.active_objects[slot] = object;
        self.mark_visibility_dirty();
        Some(slot)
    }

    /// `active-objects.md §4` eviction cascade. Runs at *acquisition* time --
    /// the spec places eviction in the allocator ("acquisition can evict a
    /// lower-priority object"), and §8.1 confirms the split: "Eviction is
    /// demand-driven - it runs when acquisition needs a slot and the range is
    /// full - and chooses a victim by class priority. Pruning is time-driven,
    /// runs every overworld turn regardless of pressure, and chooses by
    /// position alone." The per-turn sweep is the separate §8.1 prune pass
    /// ([`Self::prune_far_overworld_objects`]); the two must not be collapsed.
    /// `encounters.md §4` corroborates the trigger: on a successful spawn "the
    /// spawner acquires or evicts an active-object slot".
    ///
    /// Phases run in published order 1..=10, and within each phase the
    /// ordinary acquisition range is scanned lowest-index-up, matching the
    /// allocator's own scan discipline. Only slots whose
    /// [`active_object_slot_role`] is `OrdinaryAcquisition` are candidates, so
    /// slot 0 (the player) and the reserved slots 24..=31 can never be taken.
    /// Type byte [`ACTIVE_OBJECT_PROTECTED_TYPE_BYTE`] (`0xB5`) is the only
    /// universally protected byte-0 value and is rejected by every phase,
    /// last-resort phase 10 included.
    ///
    /// Phase 1 (the empty-slot phase) is included for completeness; the caller
    /// has already exhausted it.
    pub fn active_object_eviction_victim(&self) -> Option<usize> {
        let mut phase = ACTIVE_OBJECT_EVICTION_PHASE_FIRST;
        while phase <= ACTIVE_OBJECT_EVICTION_PHASE_LAST {
            let phase_needs_off_screen = active_object_eviction_phase_is_off_screen(phase);
            for (slot, candidate) in self.active_objects.iter().enumerate() {
                if !matches!(
                    active_object_slot_role(slot),
                    Some(ActiveObjectSlotRole::OrdinaryAcquisition)
                ) {
                    continue;
                }
                if !active_object_eviction_byte_accepted(candidate.type_byte, phase) {
                    continue;
                }
                if phase_needs_off_screen && !self.active_object_off_screen(*candidate) {
                    continue;
                }
                return Some(slot);
            }
            phase += 1;
        }
        None
    }

    /// `active-objects.md §4` off-screen gate for eviction phases 2..=5. It
    /// passes the current player globals and candidate record X/Y into the
    /// exact wrapped-byte predicate; candidate floor is deliberately ignored.
    pub fn active_object_off_screen(&self, object: ActiveObject) -> bool {
        active_object_eviction_off_screen(
            object.x as u8,
            object.y as u8,
            self.player.x as u8,
            self.player.y as u8,
        )
    }

    #[cfg(test)]
    pub fn exit_vehicle(&mut self) -> MoveOutcome {
        self.exit_vehicle_with_game_dir(None)
            .expect("vehicle exit without sidecar metadata cannot fail on file I/O")
    }

    pub fn exit_vehicle_with_game_dir(
        &mut self,
        game_dir: Option<&Path>,
    ) -> io::Result<MoveOutcome> {
        let transport = self.player.transport;
        if transport.is_foot() {
            // `commands.md §5.3`: `X-it ` is an operand-follows echo, and the
            // on-foot refusal completes that same line (`cleak/u5-spec#194`).
            // `commands.md §7`: this is also the dungeon answer - the party
            // is always on foot underground.
            if !self.complete_open_direction_echo("X-it ", XIT_ON_FOOT_REFUSAL) {
                self.message = format!("X-it {XIT_ON_FOOT_REFUSAL}");
            }
            return Ok(MoveOutcome::Blocked);
        }
        // Engine guard for a mounted transport value underground, which
        // shipped play never produces; vehicle landing below is not written
        // for dungeon cells.
        if matches!(self.area, Area::Dungeon { .. }) {
            self.message = "Not here!".to_string();
            return Ok(MoveOutcome::Blocked);
        }
        if matches!(
            transport,
            TransportState::Ship {
                sails_hoisted: true,
                ..
            }
        ) {
            self.message = SHIP_XIT_UNDER_SAIL_REFUSAL.to_string();
            return Ok(MoveOutcome::Blocked);
        }
        let Some(z) = self.current_floor() else {
            self.message = "Not here!".to_string();
            return Ok(MoveOutcome::Blocked);
        };
        let old_x = self.player.x;
        let old_y = self.player.y;
        if !self.vehicle_can_park_at_current_cell() {
            self.message = "Not here!".to_string();
            return Ok(MoveOutcome::Blocked);
        }
        let exit_position = self.vehicle_exit_current_position_if_accepted(game_dir)?;

        // doors-and-z-transitions.md §11 / vehicles.md §5: a furled-ship exit
        // without nearby foot landing falls back to launching a carried skiff.
        // The ship hull stays parked at the original cell with one fewer
        // skiff aboard, and the party becomes the launched skiff in place.
        if exit_position.is_none() {
            if let TransportState::Ship {
                sails_hoisted: false,
                skiffs,
                ..
            } = transport
            {
                // `vehicles.md §5`: the hull is parked as the furled-frigate
                // object, drawn by its own object byte.
                let parked_ship = transport
                    .parked_object(old_x, old_y, z)
                    .expect("a ship transport always parks an object");
                if skiffs > 0 {
                    let parked_with_one_less_skiff = ActiveObject {
                        aux3: skiffs - 1,
                        ..parked_ship
                    };
                    if self
                        .allocate_active_object_slot(parked_with_one_less_skiff)
                        .is_none()
                    {
                        // An internal invariant, not a player-facing path: the original
                        // prints nothing here, so neither does this. `u5-engine#19`.
                        self.message.clear();
                        return Ok(MoveOutcome::Blocked);
                    }
                    self.player.transport = TransportState::Skiff {
                        type_byte: FIRST_PLAYABLE_SKIFF_TILE,
                        tile: FIRST_PLAYABLE_SKIFF_TILE,
                    }
                    .with_facing(self.player.facing);
                    self.sail_cadence = 0;
                    self.sail_cached_direction = None;
                    self.sync_player_object();
                    self.mark_visibility_dirty();
                    self.advance_turn();
                    // Measured: the whole of the line is the completed
                    // echo `X-it ship!`; the launch itself is silent.
                    self.commit_prompt_reply("X-it ", XIT_SHIP_ARGUMENT);
                    return Ok(MoveOutcome::ExitedVehicle);
                }
                if self.special_items[SPECIAL_ITEM_MAGIC_CARPET_INDEX] > 0 {
                    if self.allocate_active_object_slot(parked_ship).is_none() {
                        self.message.clear();
                        return Ok(MoveOutcome::Blocked);
                    }
                    self.special_items[SPECIAL_ITEM_MAGIC_CARPET_INDEX] =
                        self.special_items[SPECIAL_ITEM_MAGIC_CARPET_INDEX].saturating_sub(1);
                    self.player.transport = TransportState::Carpet {
                        type_byte: FIRST_PLAYABLE_MAGIC_CARPET_TILE,
                        tile: FIRST_PLAYABLE_MAGIC_CARPET_TILE,
                    };
                    self.sail_cadence = 0;
                    self.sail_cached_direction = None;
                    self.sync_player_object();
                    self.mark_visibility_dirty();
                    self.advance_turn();
                    self.commit_prompt_reply("X-it ", XIT_SHIP_ARGUMENT);
                    return Ok(MoveOutcome::ExitedVehicle);
                }
                // `vehicles.md §5.1`: "Furled ship, any exit attempt |
                // `ship!\n`, before checking land/skiff/carpet
                // availability", and "If all three options fail, append
                // `\nNo skiffs on board!\n` after `ship!\n`, leaving a
                // blank row before the refusal."
                //
                // So the echo completes first and the refusal follows it.
                // The engine replaced the whole line with
                // `SHIP_NO_SKIFFS_WARNING`, which is the all-capitals
                // *boarding* warning and belongs to a different beat.
                let ship_row_in_slot =
                    if self.complete_open_direction_echo("X-it ", XIT_SHIP_ARGUMENT) {
                        // The echo was completed where it stood. If that was
                        // the transcript, completing it closed the row
                        // (`commands.md §5.3`), and the slot does not hold it.
                        self.message.ends_with(XIT_SHIP_ARGUMENT)
                    } else {
                        self.message = format!("X-it {XIT_SHIP_ARGUMENT}");
                        true
                    };
                // `ship!\n` carries its own feed and the refusal opens with
                // another, which is §5.1's "leaving a blank row before the
                // refusal". The `ship!` feed is only owed here while that row
                // is still open in the slot: adding it after the transcript
                // already closed the row printed a second blank row.
                if ship_row_in_slot {
                    self.message.push('\n');
                }
                self.message.push_str(XIT_NO_SKIFFS_REFUSAL);
                return Ok(MoveOutcome::Blocked);
            }
            // `vehicles.md §5.1` gives the non-ship families two different
            // refusals, and this engine had collapsed them into the
            // narrower one: "Carpet or skiff has no landing support |
            // `\nNo land nearby!\n`" is the general case, and
            // "Skiff refused over the bridge pair | `\nNot here!\n`" is
            // only the skiff standing on the bridge pair that
            // `catalogs/tile-catalog.md §3` gives as `0x48..0x49`.
            // The skiff's rejected pair is `0x6A/0x6B`, which this engine
            // already names - not the `0x48/0x49` pair a town's NPC start
            // markers use.
            let over_the_bridge_pair = matches!(transport, TransportState::Skiff { .. })
                && self.current_surface_tile().is_some_and(|tile| {
                    tile == SKIFF_XIT_REJECTED_BRIDGE_FIRST
                        || tile == SKIFF_XIT_REJECTED_BRIDGE_FIRST + 1
                });
            self.message = if over_the_bridge_pair {
                "\nNot here!\n".to_string()
            } else {
                XIT_NO_LAND_NEARBY_REFUSAL.to_string()
            };
            return Ok(MoveOutcome::Blocked);
        }

        let (x, y) = exit_position.expect("vehicle exit acceptance checked above");
        let Some(parked) = transport.parked_object(old_x, old_y, z) else {
            // An internal invariant, not a player-facing path.
            self.message.clear();
            return Ok(MoveOutcome::Blocked);
        };
        if self.allocate_active_object_slot(parked).is_none() {
            self.message.clear();
            return Ok(MoveOutcome::Blocked);
        }

        self.player.x = x;
        self.player.y = y;
        self.force_foot_transport();
        self.sync_player_object();
        self.mark_visibility_dirty();
        self.advance_turn();
        // `vehicles.md §5.1`: a furled-ship exit onto land "adds no further
        // success text"; the hull/skiff warnings belong to B-Board (§4).
        self.message = format!("{}!", transport.kind_name());
        Ok(MoveOutcome::ExitedVehicle)
    }

    pub fn vehicle_can_park_at_current_cell(&self) -> bool {
        if !self.player.transport.is_balloon() {
            return true;
        }
        let Some(tile) = self.current_surface_tile() else {
            return false;
        };
        !is_mountain_tile(tile) && !is_wall_or_closed_door_tile(tile)
    }

    pub fn toggle_sails(&mut self) -> MoveOutcome {
        let TransportState::Ship {
            type_byte,
            tile,
            sails_hoisted,
            hull,
            skiffs,
        } = self.player.transport
        else {
            self.message = "What?".to_string();
            return MoveOutcome::Blocked;
        };
        let next = !sails_hoisted;
        self.player.transport = TransportState::Ship {
            type_byte,
            tile,
            sails_hoisted: next,
            hull,
            skiffs,
        };
        self.sail_cadence = 0;
        // `vehicles.md` "Ship Sails": furling makes the ship "manually
        // handled" and "wind-driven drift should not advance the ship while
        // furled", so the toggle drops the cache the auto-advance route
        // reads. Hoisting starts with none, since the cache is written by a
        // movement command.
        self.sail_cached_direction = None;
        self.advance_turn();
        self.message = if next {
            YELL_SAILS_HOISTED_MESSAGE.to_string()
        } else {
            YELL_SAILS_FURLED_MESSAGE.to_string()
        };
        MoveOutcome::SailToggled
    }

    pub fn yell_command(&mut self, word: Option<&str>) -> MoveOutcome {
        let scene_byte = self.current_scene_byte();
        let aboard_frigate = matches!(self.player.transport, TransportState::Ship { .. });
        if yell_routes_to_ship_sails(scene_byte, aboard_frigate) {
            return self.toggle_sails();
        }

        let Some(word) = word else {
            self.message = yell_prompt_message();
            return MoveOutcome::PromptDeclined;
        };
        let word = Self::normalize_yell_word(word);
        // The typed word belongs on the colon row the prompt opened, not
        // on a row under it: a capture reads `>Yell what?` and then a
        // single `:HELLO`. `cleak/u5-engine#5`.
        self.commit_typed_free_text_line(":", &word);
        if word.is_empty() {
            self.message = YELL_NOTHING_SAID_MESSAGE.to_string();
            self.advance_turn();
            return MoveOutcome::Used;
        }

        self.advance_turn();
        match yell_input_context(scene_byte) {
            YellInputContext::WordOfPower => {
                if let Some((word_index, seal)) = word_of_power_seal_prefix_match(&word) {
                    // `audio.md §8.4`: the recognized-Word effect occurs before
                    // the location-specific success test, so a known Word
                    // spoken at the wrong place is still audible and visible.
                    self.emit_major_flash();
                    let outcome = self.open_word_of_power_seal(word_index, seal);
                    // `commands.md` §11.1: "A recognised word immediately
                    // prints the uttered-word result and plays the shared
                    // low-rumble / full-viewport flash presentation effect,
                    // before any location test." The result is that line
                    // alone. This engine narrated the whole thing instead -
                    // naming the word and its dungeon, and describing the
                    // rumble and flash in prose - which is presentation the
                    // player hears and sees, not text the original prints.
                    // Measured 2026-09-11 (`word-of-power-audio/dosbox-word`):
                    // the original's rows read `A word of power` / `is
                    // uttered`, a blank row, then `No effect!`.
                    self.diagnostics.push(format!(
                        "Yelled {word}, the Word of Power for {}; {}",
                        seal.dungeon,
                        Self::word_of_power_presentation_message()
                    ));
                    let utterance = crate::commands::WORD_OF_POWER_UTTERED_MESSAGE.to_string();
                    if let WordOfPowerTargetOutcome::RuinedShrine { x, y } = outcome {
                        self.active_shrine_restoration =
                            Some(ShrineRestorationSession::new(word_index, x, y, utterance));
                        self.message = self.render_active_shrine_restoration();
                        return MoveOutcome::Used;
                    }
                    // The seal-toggle outcomes' text is not published and no
                    // capture of this engine's suite reaches one, so they add
                    // nothing rather than an invented sentence; the mutation
                    // itself is the observable. The no-neighbour outcome takes
                    // the measured `No effect!` under a blank row.
                    let context = match outcome {
                        WordOfPowerTargetOutcome::EntranceToggled { open, .. } => {
                            self.diagnostics.push(format!(
                                "Word of power toggled a dungeon entrance; open={open}"
                            ));
                            ""
                        }
                        WordOfPowerTargetOutcome::RuinedShrine { .. } => unreachable!(),
                        // Step 1 "additionally prints the no-effect result"
                        // only when no neighbour qualifies; step 3's wrong
                        // coordinate "still produces the utterance feedback
                        // ... but changes nothing" (playtest G30).
                        WordOfPowerTargetOutcome::NoQualifyingNeighbor => {
                            crate::commands::WORD_OF_POWER_NO_EFFECT_TAIL
                        }
                        WordOfPowerTargetOutcome::WrongCoordinate { x, y } => {
                            self.diagnostics.push(format!(
                                "Word of power: sealed neighbour ({x}, {y}) is not this word's entrance"
                            ));
                            ""
                        }
                    };
                    self.message = format!("{utterance}{context}");
                    return MoveOutcome::Used;
                }
            }
            YellInputContext::ShadowlordName => {
                if let Some(index) = Self::shadowlord_name_index(&word) {
                    let shadowlord =
                        Self::shadowlord_title_for_index(index).unwrap_or("Shadowlord");
                    // `commands.md` §11 (added 2026-10-16, u5-spec#298 Q11):
                    // after the install the handler runs three steps in
                    // order - it prints `\nA_shadowlord_appears\n`, runs the
                    // blocking envelope `(2, 2000, 30000, 1, 10400)`
                    // (`audio.md §8.3`), then fades the actor in with the
                    // shared single-cell converge on tile `0x1FC` at
                    // viewport column 5, row 3, the actor byte parked at
                    // `0x16` and restored to `0xFC` afterwards. No runtime
                    // channel carries the converge (see the monster summon
                    // flash in `combat_frame.rs`), so it is recorded in the
                    // diagnostics only; the actor byte ends as `0xFC`.
                    self.message = if let Some(slot) = self.place_shadowlord_name_encounter(index) {
                        self.emit_sound_effect(SoundEffect::ShadowlordYellSummon);
                        self.diagnostics.push(format!(
                            "Yelled {word}, the name of {shadowlord}; summoned into active-object slot {slot}; converge tile 0x{:03X} at viewport (5, 3)",
                            crate::commands::SHADOWLORD_YELL_SUMMON_CONVERGE_TILE
                        ));
                        format!("\n{}\n", crate::commands::SHADOWLORD_YELL_SUMMON_LINE)
                    } else {
                        format!("\n{YELL_NO_EFFECT_MESSAGE}")
                    };
                    return MoveOutcome::Used;
                }
            }
            YellInputContext::NoEffect => {}
        }

        // The word's colon echo stays on screen above the result.
        self.message = format!("\n{YELL_NO_EFFECT_MESSAGE}");
        MoveOutcome::Used
    }

    pub fn normalize_yell_word(value: &str) -> String {
        value
            .trim()
            .chars()
            .take(30)
            .flat_map(char::to_uppercase)
            .collect()
    }

    pub fn word_of_power_dungeon(word: &str) -> Option<&'static str> {
        match word {
            "FALLAX" => Some("Deceit"),
            "VILIS" => Some("Despise"),
            "INOPIA" => Some("Destard"),
            "MALUM" => Some("Wrong"),
            "AVIDUS" => Some("Covetous"),
            "INFAMA" => Some("Shame"),
            "IGNAVUS" => Some("Hythloth"),
            "VERAMOCOR" => Some("Doom"),
            _ => None,
        }
    }

    pub const fn word_of_power_presentation_message() -> &'static str {
        "A low rumble and full-viewport flash answer the word."
    }

    pub fn render_active_shrine_restoration(&self) -> String {
        self.active_shrine_restoration
            .as_ref()
            .map(|session| format!("{}{}", session.transcript, session.buffer))
            .unwrap_or_default()
    }

    pub fn step_active_shrine_restoration(
        &mut self,
        key: char,
        suffix: &str,
    ) -> Option<MoveOutcome> {
        let Some(mut session) = self.active_shrine_restoration.take() else {
            return None;
        };
        if key == '\u{1b}' {
            session.buffer.clear();
            self.message = format!("{}{}", session.transcript, session.buffer);
            self.active_shrine_restoration = Some(session);
            return None;
        }
        if matches!(key, '\u{8}' | '\u{7f}') {
            session.buffer.pop();
            self.message = format!("{}{}", session.transcript, session.buffer);
            self.active_shrine_restoration = Some(session);
            return None;
        }

        let mut response = String::new();
        // `input.md §8`: a byte that is not printable ASCII - a function
        // key included (`§5`) - is discarded, not taken as a typed line.
        if !matches!(key, '\r' | '\n') && key.is_ascii() && !key.is_control() {
            response.push(key);
        }
        response.push_str(suffix);
        session.buffer.extend(
            response
                .chars()
                .filter(|ch| ch.is_ascii() && !ch.is_control())
                .take(SHRINE_RESTORATION_INPUT_MAX_LEN.saturating_sub(session.buffer.len())),
        );
        if response.is_empty() && !matches!(key, '\r' | '\n') {
            self.message = format!("{}{}", session.transcript, session.buffer);
            self.active_shrine_restoration = Some(session);
            return None;
        }

        let virtue = ShrineVirtue::from_index(session.word_index)
            .expect("Word-of-Power index is a standard virtue index");
        let required = if session.response_index == 0 {
            virtue.name()
        } else {
            virtue.mantra()
        };
        // `karma.md §7.1` / `commands.md §11.3`: the shared matcher, restart
        // quirk included - `hhonesty` and `aahm` fail. A plain substring
        // search accepted both (playtest E62).
        let matches = !session.buffer.is_empty()
            && crate::shrine_virtue::shrine_answer_matches(&session.buffer, required);
        session.all_responses_match &= matches;
        session.transcript.push_str(&session.buffer);
        session.buffer.clear();

        if session.response_index < 3 {
            session.response_index += 1;
            session
                .transcript
                .push_str(SHRINE_RESTORATION_MANTRA_PROMPT);
            self.message = session.transcript.clone();
            self.active_shrine_restoration = Some(session);
            return None;
        }

        let coordinate_matches = session.word_index != ShrineVirtue::Spirituality.index()
            && WORLD_SHRINE_COORDINATES.get(session.word_index).copied()
                == Some((session.target_x, session.target_y));
        let target_index = world_cell_index(session.target_x, session.target_y);
        let target_still_ruined =
            self.grid.get(target_index).copied() == Some(WORLD_RUINED_SHRINE_TILE);
        if session.all_responses_match && coordinate_matches && target_still_ruined {
            self.shrine_ruin_flags[session.word_index] &= !SAVE_QUEST_TILE_FLAG_HIGH_BIT;
            self.grid[target_index] = WORLD_SHRINE_TILE;
            // `karma.md §7.1` Success row: the banner, then the shared
            // flash/rumble a second time, "which adds no text" - the prose
            // description is a diagnostic, as on the utterance path.
            session
                .transcript
                .push_str(SHRINE_RESTORATION_SUCCESS_BANNER);
            self.diagnostics.push(format!(
                "Shrine restored; {}",
                Self::word_of_power_presentation_message()
            ));
            // `audio.md §8.4`: a successful ruined-shrine restoration invokes
            // the shared full-viewport flash again at its own success
            // boundary — a second invocation after the recognized-Word flash.
            self.emit_major_flash();
            let _ = self.refresh_world_live_chunks_for_current_area();
            self.mark_visibility_dirty();
        } else {
            session.transcript.push('\n');
        }
        self.message = session.transcript;
        Some(MoveOutcome::Used)
    }

    pub fn open_word_of_power_seal(
        &mut self,
        word_index: usize,
        seal: WordOfPowerSeal,
    ) -> WordOfPowerTargetOutcome {
        if !matches!(self.area, Area::World { .. }) {
            return WordOfPowerTargetOutcome::NoQualifyingNeighbor;
        }
        let adjacent = [
            (self.player.x.wrapping_sub(1) % WORLD_SIDE, self.player.y),
            (self.player.x, (self.player.y + 1) % WORLD_SIDE),
            ((self.player.x + 1) % WORLD_SIDE, self.player.y),
            (self.player.x, self.player.y.wrapping_sub(1) % WORLD_SIDE),
        ];
        let Some((x, y, tile)) = adjacent.into_iter().find_map(|(x, y)| {
            let tile = self.world_live_tile_at(x, y);
            matches!(
                tile,
                value if value == seal.unsealed_tile
                    || value == WORD_OF_POWER_SEALED_TILE
                    || value == WORLD_RUINED_SHRINE_TILE
            )
            .then_some((x, y, tile))
        }) else {
            return WordOfPowerTargetOutcome::NoQualifyingNeighbor;
        };
        if tile == WORLD_RUINED_SHRINE_TILE {
            return WordOfPowerTargetOutcome::RuinedShrine { x, y };
        }
        if (x, y) != (seal.x, seal.y) {
            return WordOfPowerTargetOutcome::WrongCoordinate { x, y };
        }
        let open = tile == WORD_OF_POWER_SEALED_TILE;
        self.grid[world_cell_index(x, y)] = if open {
            seal.unsealed_tile
        } else {
            WORD_OF_POWER_SEALED_TILE
        };
        self.word_of_power_seal_flags[word_index] ^= SAVE_QUEST_TILE_FLAG_HIGH_BIT;
        let _ = self.refresh_world_live_chunks_for_current_area();
        self.mark_visibility_dirty();
        WordOfPowerTargetOutcome::EntranceToggled { x, y, open }
    }

    pub fn shadowlord_name(word: &str) -> Option<&'static str> {
        Self::shadowlord_name_index(word).and_then(Self::shadowlord_title_for_index)
    }

    /// `commands.md §11.3` (u5-spec#298 Q34): "Yell's Shadowlord branch
    /// tests its three capitalised names in index order and the first match
    /// wins", each through the shared text matcher, so the name may sit
    /// anywhere in the typed line (subject to the matcher's restart quirk).
    /// The keywords are `catalogs/quest-graph.md §5`'s Name column.
    pub fn shadowlord_name_index(word: &str) -> Option<usize> {
        [
            (SHADOWLORD_NAME_FAULINEI, SHADOWLORD_FALSEHOOD_INDEX),
            (SHADOWLORD_NAME_ASTAROTH, SHADOWLORD_HATRED_INDEX),
            (SHADOWLORD_NAME_NOSFENTOR, SHADOWLORD_COWARDICE_INDEX),
        ]
        .into_iter()
        .find(|(name, _)| crate::commands::shared_text_matcher_matches(word, name))
        .map(|(_, index)| index)
    }

    pub fn shadowlord_title_for_index(index: usize) -> Option<&'static str> {
        match index {
            SHADOWLORD_FALSEHOOD_INDEX => Some("Falsehood"),
            SHADOWLORD_HATRED_INDEX => Some("Hatred"),
            SHADOWLORD_COWARDICE_INDEX => Some("Cowardice"),
            _ => None,
        }
    }

    /// The virtue-opposition word as the "air of" line spells it.
    ///
    /// `town-mode.md §13` names the three as "an air of falsehood, of hatred,
    /// or of cowardice", lower case; [`Self::shadowlord_title_for_index`] is
    /// the capitalised title used elsewhere.
    pub fn shadowlord_air_word_for_index(index: usize) -> Option<&'static str> {
        match index {
            SHADOWLORD_FALSEHOOD_INDEX => Some("falsehood"),
            SHADOWLORD_HATRED_INDEX => Some("hatred"),
            SHADOWLORD_COWARDICE_INDEX => Some("cowardice"),
            _ => None,
        }
    }

    /// `town-mode.md §13`: the single in-town notice a settlement hosting a
    /// Shadowlord prints after its map is drawn, and the per-Shadowlord row
    /// Stonegate's separate entry presentation prints.
    ///
    /// Measured 2026-09-11 (`stonegate-trapdoor-audio/arrival`): the rows read
    /// `An air of` / `<virtue> doth` / `surround thee...`, so the sentence
    /// ends in a three-dot ellipsis rather than a full stop.
    ///
    /// **The row breaks are authored, not wrapped.** All three rows take the
    /// same shape in the capture, including hatred's - and `An air of hatred`
    /// is exactly sixteen characters, so a wrap would have kept it whole. The
    /// original's own combat captures show it doing precisely that with
    /// `Iolo, armed with` and `with Long Sword:`
    /// (`result_lines_conformance.rs`), so the difference here cannot come
    /// from the wrap rule. The breaks are carried as explicit newlines to
    /// reproduce the authored layout.
    pub fn shadowlord_air_line_for_index(index: usize) -> Option<String> {
        Self::shadowlord_air_word_for_index(index)
            .map(|word| format!("An air of\n{word} doth\nsurround thee..."))
    }

    /// `town-mode.md §13`, Stonegate's separate entry presentation: it
    /// "prints one such line per still-living Shadowlord regardless of where
    /// each is hiding", walking the three runtime slots and skipping any
    /// holding the vanquished marker.
    ///
    /// Measured in the same capture with all three alive: the rows arrive as
    /// cowardice, then hatred, then falsehood - the descending slot order -
    /// each followed by a blank row. The spec says "walks the three
    /// Shadowlord runtime slots" without naming a direction, so the order
    /// here is the measurement.
    pub fn stonegate_entry_presentation_lines(&self) -> Vec<String> {
        (0..SHADOWLORD_COUNT)
            .rev()
            // `town-mode.md §13`: "values with the vanquished marker are
            // skipped, while any living value selects that Shadowlord's
            // atmospheric row." The test is *not vanquished*, not the
            // `1..8` hideout range that `shadowlord_alive` applies: a slot
            // holding `0` means "not yet placed" and, per `time.md §7`, is
            // "neither in a town nor vanquished". Measured 2026-09-11
            // (`stonegate-trapdoor-audio/arrival`) on a save whose slots are
            // all unplaced - the original prints all three rows, and the
            // range test printed none.
            .filter(|index| !self.shadowlord_vanquished(*index))
            .filter_map(Self::shadowlord_air_line_for_index)
            .collect()
    }

    pub fn shadowlord_object_tile_for_index(index: usize) -> Option<u8> {
        (index < SHADOWLORD_COUNT).then_some(SHADOWLORD_ACTOR_TILE)
    }

    pub fn shadowlord_name_encounter_object(
        &self,
        index: usize,
        x: usize,
        y: usize,
        z: i8,
    ) -> Option<ActiveObject> {
        let tile = Self::shadowlord_object_tile_for_index(index)?;
        let dx = x as i8 - self.player.x as i8;
        let dy = y as i8 - self.player.y as i8;
        Some(ActiveObject {
            type_byte: tile,
            tile,
            x,
            y,
            z,
            phase: active_object_phase_toward_player(dx, dy),
            aux1: 0,
            aux3: 0,
        })
    }

    pub fn is_shadowlord_actor(object: ActiveObject) -> bool {
        !object.is_empty() && object.type_byte == SHADOWLORD_ACTOR_TILE
    }

    pub fn shadowlord_name_encounter_present(&self, index: usize) -> bool {
        if self.summoned_shadowlord != Some(index) {
            return false;
        }
        let Some(floor) = self.current_floor() else {
            return false;
        };
        self.active_objects
            .iter()
            .copied()
            .skip(1)
            .any(|object| object.z == floor && Self::is_shadowlord_actor(object))
    }

    /// `commands.md §11` / `town-mode.md §13` shared one-at-a-time
    /// predicate. Shadowlord identity is stored separately; any live
    /// `0xFC` actor blocks another summon or resident installation.
    pub fn shadowlord_actor_present(&self) -> bool {
        self.active_objects
            .iter()
            .skip(1)
            .any(|object| !object.is_empty() && object.type_byte == SHADOWLORD_ACTOR_TILE)
    }

    pub fn matching_shadowlord_name_encounter_north(&self, index: usize) -> bool {
        let Some(floor) = self.current_floor() else {
            return false;
        };
        let Some(y) = self.player.y.checked_sub(1) else {
            return false;
        };
        let x = self.player.x;
        self.active_objects.iter().copied().skip(1).any(|object| {
            object.z == floor
                && object.x == x
                && object.y == y
                && self.summoned_shadowlord == Some(index)
                && Self::is_shadowlord_actor(object)
        })
    }

    /// `commands.md §11` Shadowlord-name contexts: the summon needs "that
    /// Shadowlord's slot is not vanquished", and `quest-graph.md §5` makes
    /// vanquished the high bit ("Consumers test the high bit"). A `0` slot
    /// ("Not yet placed") is therefore summonable; gating on the living
    /// `1..=8` range refused it (playtest E64).
    pub fn place_shadowlord_name_encounter(&mut self, index: usize) -> Option<usize> {
        let Area::Town { scene, .. } = self.area else {
            return None;
        };
        if !matches!(
            scene.byte,
            SCENE_THE_LYCAEUM | SCENE_EMPATH_ABBEY | SCENE_SERPENTS_HOLD
        ) || self.shadowlord_reads_vanquished(index)
            || self.player.y < 2
            || self.shadowlord_actor_present()
        {
            return None;
        }
        let z = self.current_floor()?;
        let (x, y) = (self.player.x, self.player.y - 2);
        let object = self.shadowlord_name_encounter_object(index, x, y, z)?;
        let slot = self.allocate_highest_empty_active_object_slot(object)?;
        self.summoned_shadowlord = Some(index);
        self.mark_visibility_dirty();
        Some(slot)
    }

    /// Highest-empty active-object discipline used by name/Yell summons.
    /// Slot zero remains the player; a full table has no empty slot.
    pub fn allocate_highest_empty_active_object_slot(
        &mut self,
        object: ActiveObject,
    ) -> Option<usize> {
        self.active_objects.resize(OOL_SLOTS, ActiveObject::empty());
        let slot = (1..OOL_SLOTS)
            .rev()
            .find(|slot| self.active_objects[*slot].is_empty())?;
        // An off-floor NPC may retain a link to a descriptor that is empty on
        // the current floor. Acquisition transfers ownership of that record;
        // detach the stale link so the next schedule pass cannot reclaim and
        // overwrite the summoned actor.
        for npc in &mut self.npcs {
            if npc.active_object == Some(slot) {
                npc.active_object = None;
            }
        }
        self.active_objects[slot] = object;
        Some(slot)
    }

    /// `town-mode.md §13` entry-time resident selector and actor install.
    /// `Some((None, index))` records a host whose actor was rejected by the
    /// shared one-at-a-time gate; `Some((Some(slot), index))` installed it.
    pub fn install_shadowlord_entry_encounter(&mut self) -> Option<(Option<usize>, usize)> {
        self.resident_shadowlord = None;
        if self.player.y == SHADOWLORD_TOWN_ENTRY_SKIP_Y {
            return None;
        }
        let Area::Town { scene, floor } = self.area else {
            return None;
        };
        let shadowlord_index = self
            .shadowlord_hideouts
            .iter()
            .position(|hideout| *hideout == scene.byte)?;
        self.resident_shadowlord = Some(shadowlord_index);
        let _ = self.apply_resident_shadowlord_blight_with_seed(host_clock_prng_seed_now());
        if self.shadowlord_actor_present() {
            return Some((None, shadowlord_index));
        }

        let active_slot = self.insert_resident_shadowlord_npc(scene, floor)?;
        // `town-mode.md §13` (u5-spec#300 Q4): the sweep runs after the
        // party-capability check, and a party with nobody able to act and
        // nobody asleep stops entry there, "no resident-entry sweep".
        if self.shadowlord_notice_capability_gate_passes() {
            let _ = self.apply_resident_shadowlord_npc_sweep(shadowlord_index);
        }
        self.mark_visibility_dirty();
        Some((active_slot, shadowlord_index))
    }

    fn insert_resident_shadowlord_npc(&mut self, scene: Scene, floor: i8) -> Option<Option<usize>> {
        let y = shadowlord_town_install_row(scene.byte)?;
        let npc_slot = (1..OOL_SLOTS)
            .rev()
            .find(|slot| !self.npcs.iter().any(|npc| npc.slot == *slot))
            .unwrap_or(OOL_SLOTS - 1);
        if let Some(existing_index) = self.npcs.iter().position(|npc| npc.slot == npc_slot) {
            if let Some(active_slot) = self.npcs[existing_index].active_object {
                self.free_active_object_slot(active_slot);
            }
            self.npcs.remove(existing_index);
        }
        self.npcs.push(RuntimeNpc::from_resident_shadowlord(
            npc_slot,
            SHADOWLORD_TOWN_INSTALL_X,
            y,
            self.clock.hour,
        ));
        self.npcs.sort_by_key(|npc| npc.slot);

        let active_slot = if floor == 0 {
            let npc_index = self
                .npcs
                .iter()
                .position(|npc| npc.slot == npc_slot)
                .expect("resident Shadowlord descriptor was just inserted");
            self.sync_npc_active_object(npc_index, 0);
            self.npcs[npc_index].active_object
        } else {
            None
        };
        Some(active_slot)
    }

    /// `town-mode.md §3` resident-only deterministic terrain walk followed
    /// by the published fresh host-clock state replacement.
    pub fn apply_resident_shadowlord_blight_with_seed(
        &mut self,
        trailing_host_seed: u16,
    ) -> Option<usize> {
        self.resident_shadowlord?;
        let rewritten = apply_shadowlord_blight(
            &mut self.grid,
            self.clock.day,
            &mut self.prng_state,
            trailing_host_seed,
        );
        self.mark_visibility_dirty();
        Some(rewritten)
    }

    /// `town-mode.md §13`: consume one coin draw for every roster index,
    /// then apply the host-specific destructive schedule/dialogue sweep.
    pub fn apply_resident_shadowlord_npc_sweep(
        &mut self,
        shadowlord_index: usize,
    ) -> (usize, usize) {
        if !matches!(
            shadowlord_index,
            SHADOWLORD_HATRED_INDEX | SHADOWLORD_COWARDICE_INDEX
        ) {
            return (0, 0);
        }
        let draws = std::array::from_fn(|_| self.random_range_u8(0, 1));
        self.apply_resident_shadowlord_npc_sweep_with_draws(shadowlord_index, &draws)
    }

    /// Deterministic form of the resident sweep. The fixed-slot-4 type test is
    /// intentionally reproduced for compatibility with the published defect.
    pub fn apply_resident_shadowlord_npc_sweep_with_draws(
        &mut self,
        shadowlord_index: usize,
        draws: &[u8; NPC_SLOTS_PER_SUB_MAP],
    ) -> (usize, usize) {
        let fixed_type_passes = self
            .npcs
            .iter()
            .find(|npc| npc.slot == 4)
            .is_some_and(|npc| {
                (TOWN_NPC_ORDINARY_TYPE_FIRST..=TOWN_NPC_ORDINARY_TYPE_LAST)
                    .contains(&npc.type_byte)
            });
        if !fixed_type_passes {
            return (0, 0);
        }

        let mut pursued = 0;
        let mut cowering = 0;
        for roster_index in 0..NPC_SLOTS_PER_SUB_MAP {
            if draws[roster_index] != 0 {
                continue;
            }
            let Some(index) = self.npcs.iter().position(|npc| npc.slot == roster_index) else {
                continue;
            };
            if !self.npcs[index].has_nonzero_schedule_time_boundary() {
                continue;
            }
            match shadowlord_index {
                SHADOWLORD_HATRED_INDEX => {
                    self.npcs[index].force_town_pursuit();
                    self.npcs[index].dialog_id = TOWN_NPC_BRUSHOFF_DIALOG_ID;
                    self.record_town_npc_mutation(index);
                    pursued += 1;
                }
                SHADOWLORD_COWARDICE_INDEX => {
                    let _ = self.npcs[index].force_town_flight();
                    self.npcs[index].dialog_id = TOWN_NPC_COWERING_DIALOG_ID;
                    self.record_town_npc_mutation(index);
                    cowering += 1;
                }
                _ => {}
            }
        }
        (pursued, cowering)
    }

    /// Restore the resident Shadowlord's logical high-index NPC descriptor
    /// after an in-town floor reload rebuilt the ordinary roster from disk.
    pub fn restore_resident_shadowlord_after_floor_reload(&mut self) -> Option<usize> {
        let Area::Town { scene, floor } = self.area else {
            return None;
        };
        self.resident_shadowlord = None;
        if self.player.y == SHADOWLORD_TOWN_ENTRY_SKIP_Y {
            return None;
        }
        self.resident_shadowlord = self
            .shadowlord_hideouts
            .iter()
            .position(|hideout| *hideout == scene.byte);
        self.resident_shadowlord?;
        let _ = self.apply_resident_shadowlord_blight_with_seed(host_clock_prng_seed_now());
        self.insert_resident_shadowlord_npc(scene, floor).flatten()
    }

    pub fn shadowlord_slot_is_living(value: u8) -> bool {
        (SHADOWLORD_HIDEOUT_MIN..=SHADOWLORD_HIDEOUT_MAX).contains(&value)
    }

    /// `time.md §7`: "the daily walker skips any slot whose high bit is set".
    /// The skip test is the high bit, not the living `1..8` range, because a
    /// slot holding `0` means "not yet placed" — neither in a town nor
    /// vanquished — and "the reroll walker rewrites it on the first day
    /// rollover". Only a vanquished `0xFF` slot is sticky.
    pub fn shadowlord_slot_is_rerollable(value: u8) -> bool {
        value & SAVE_QUEST_TILE_FLAG_HIGH_BIT == 0
    }

    pub fn shadowlord_slot_is_vanquished(value: u8) -> bool {
        value == SHADOWLORD_VANQUISHED
    }

    /// `quest-graph.md §5`: "Consumers test the high bit, so any value with
    /// the high bit set reads as vanquished." The summon and destruction
    /// gates read the slot this way; `0` ("Not yet placed") is not
    /// vanquished. An out-of-range index reads as vanquished, so neither
    /// gate opens for it.
    pub fn shadowlord_reads_vanquished(&self, index: usize) -> bool {
        self.shadowlord_hideouts
            .get(index)
            .is_none_or(|value| value & SAVE_QUEST_TILE_FLAG_HIGH_BIT != 0)
    }

    pub fn shadowlord_alive(&self, index: usize) -> bool {
        self.shadowlord_hideouts
            .get(index)
            .copied()
            .is_some_and(Self::shadowlord_slot_is_living)
    }

    pub fn shadowlord_vanquished(&self, index: usize) -> bool {
        self.shadowlord_hideouts
            .get(index)
            .copied()
            .is_some_and(Self::shadowlord_slot_is_vanquished)
    }

    pub fn all_shadowlords_vanquished(&self) -> bool {
        self.shadowlord_hideouts
            .iter()
            .copied()
            .all(Self::shadowlord_slot_is_vanquished)
    }

    pub fn vanquish_shadowlord(&mut self, index: usize) -> bool {
        // The same "not vanquished" test as the summon gate: a summon from a
        // not-yet-placed `0` slot must be destroyable too.
        if self.shadowlord_reads_vanquished(index) {
            return false;
        }
        self.shadowlord_hideouts[index] = SHADOWLORD_VANQUISHED;
        if let Some(slot) = shadowlord_stonegate_npc_slot(index) {
            let scene = Scene::new(STONEGATE_SCENE_BYTE)
                .expect("Stonegate scene byte is a valid town scene");
            self.mark_removed_town_npc_once(scene, slot);
        }
        true
    }

    /// `town-mode.md §13`, Stonegate's separate entry presentation: after the
    /// location is drawn it "walks the three Shadowlord runtime slots and,
    /// for every Shadowlord that is still alive, prints the corresponding
    /// \"air of\" virtue-opposition line", skipping the vanquished marker.
    ///
    /// This used to return an engine summary - `Stonegate entry: Sceptre
    /// prelude; air of Hatred.` - which is a sentence the original never
    /// prints. The rows themselves are the presentation. R590 (u5-spec#300
    /// Q4) withdrew the "Sceptre-gated prelude row" outright.
    pub fn stonegate_entry_presentation_message(&self) -> Option<String> {
        let Area::Town { scene, .. } = self.area else {
            return None;
        };
        if scene.byte != STONEGATE_SCENE_BYTE {
            return None;
        }
        if !self.shadowlord_notice_capability_gate_passes() {
            return None;
        }
        let lines = self.stonegate_entry_presentation_lines();
        (!lines.is_empty()).then(|| lines.join("\n\n"))
    }

    /// `town-mode.md §13` (u5-spec#300 Q4, Q17): town entry runs the shared
    /// party-capability check before the notice, and "If the capability
    /// check finds nobody in the party able to act and nobody asleep, entry
    /// stops at that check: no notice, no tone and no resident-entry sweep".
    pub fn shadowlord_notice_capability_gate_passes(&self) -> bool {
        self.party_capability() != PartyCapability::Defeated
    }

    /// `town-mode.md §13` (u5-spec#300 Q4, R590): "when the party's Sceptre
    /// flag is set ..., the setup path silently removes roster slot 9, the
    /// Sceptre item figure that stands on the centre cell ringed by the
    /// trapdoors. Nothing is printed and nothing sounds." No removal mark is
    /// written: the figure "is placed again by the roster load of the next
    /// entry after a Shadow Lord fight has reclaimed the Sceptre and cleared
    /// the flag". Static reading in the spec, not executed.
    pub fn remove_stonegate_sceptre_figure_on_entry(&mut self) -> bool {
        let Area::Town { scene, .. } = self.area else {
            return false;
        };
        if scene.byte != STONEGATE_SCENE_BYTE
            || self.special_items[SPECIAL_ITEM_SCEPTRE_LB_INDEX] == 0
        {
            return false;
        }
        let Some(index) = self
            .npcs
            .iter()
            .position(|npc| npc.slot == STONEGATE_SCEPTRE_FIGURE_NPC_SLOT)
        else {
            return false;
        };
        if let Some(slot) = self.npcs[index].active_object {
            self.free_active_object_slot(slot);
        }
        self.npcs.remove(index);
        self.mark_visibility_dirty();
        true
    }

    pub fn append_stonegate_entry_presentation_message(&mut self) {
        let Area::Town { scene, .. } = self.area else {
            return;
        };
        if scene.byte != STONEGATE_SCENE_BYTE {
            return;
        }
        // `town-mode.md §13` (u5-spec#300 Q4, R590): "The published
        // \"Sceptre-gated prelude row\" does not exist"; the Sceptre flag
        // gates only the silent figure removal of
        // [`Self::remove_stonegate_sceptre_figure_on_entry`], which entry
        // setup runs before this.
        if !self.shadowlord_notice_capability_gate_passes() {
            return;
        }
        for index in (0..SHADOWLORD_COUNT).rev() {
            if self.shadowlord_alive(index) {
                let title = Self::shadowlord_title_for_index(index).unwrap_or("Shadowlord");
                self.diagnostics
                    .push(format!("Stonegate entry: {title} still alive"));
            }
        }
        if self.stonegate_entry_presentation_message().is_none() {
            return;
        }
        // `audio.md §8.6` / `town-mode.md §13` (u5-spec#300 Q4/Q17): the
        // producer walks the slots and, for each living Shadowlord, "prints
        // that Shadowlord's notice and plays its tone"; the tone runs "After
        // the notice text prints" and "blocks", so each tone finishes before
        // the next notice's text. The first notice and its tone go out now;
        // every later notice is staged behind the previous tone's length and
        // plays its own tone as it is released
        // ([`Self::release_stonegate_notice_beat_audio`]).
        let mut lines = self.stonegate_entry_presentation_lines().into_iter();
        let Some(first) = lines.next() else {
            return;
        };
        // This has to go through the transcript, not the bare message slot.
        // The gameplay window renders from `message_entries()`, so a line
        // written straight into `self.message` during scene load never
        // reached the window at all - which is why the `Stonegate entry:`
        // summary this replaced was never visible in a capture despite
        // having tests over it.
        let composed = match self.message.is_empty() {
            true => first,
            false => format!("{}\n\n{first}", self.message),
        };
        self.emit_message_line(composed);
        self.emit_sound_effect(SoundEffect::ShadowlordPresenceNotice);
        let tone_secs = SoundEffect::ShadowlordPresenceNotice
            .program(&mut crate::audio::RumbleJitter::default())
            .duration(true)
            .as_secs_f64();
        for line in lines {
            // Each notice "carries its own leading blank row". The notice
            // before it ended on `surround thee...` with no feed of its own
            // here, so the emit above left that row open mid-line; say so
            // rather than infer it. The first feed closes that row and the
            // second spends the blank.
            self.staged_narration.push_secs(tone_secs, format!("\n\n{line}"));
        }
        self.message_row_open_mid_line = true;
    }

    /// The tone of a staged Stonegate notice, played as its text is released
    /// (`audio.md §8.6`: "After the notice text prints, run `(1, 2000, 60000,
    /// 1, 6600)`"). Only Stonegate stages these notices; a hideout town's
    /// single notice plays its tone at once.
    pub(crate) fn release_stonegate_notice_beat_audio(&mut self, text: &str) {
        let Some(line) = text.strip_prefix("\n\n") else {
            return;
        };
        if (0..SHADOWLORD_COUNT)
            .filter_map(Self::shadowlord_air_line_for_index)
            .any(|notice| notice == line)
        {
            self.emit_sound_effect(SoundEffect::ShadowlordPresenceNotice);
        }
    }

    pub fn current_shadowlord_hideout_id(&self) -> Option<u8> {
        let Area::Town { scene, .. } = self.area else {
            return None;
        };
        (SHADOWLORD_HIDEOUT_MIN..=SHADOWLORD_HIDEOUT_MAX)
            .contains(&scene.byte)
            .then_some(scene.byte)
    }

    pub fn reroll_shadowlord_hideouts(&mut self) -> usize {
        self.reroll_shadowlord_hideouts_excluding(self.current_shadowlord_hideout_id())
    }

    /// `time.md §7`: "For each slot whose high bit is clear, the midnight
    /// pass draws a candidate id uniformly from `1..8` inclusive and rejects
    /// it when either of these holds, then draws again: the candidate equals
    /// the party's current scene byte, or the candidate equals the value
    /// currently stored in **any** of the three slots, including the slot
    /// being rerolled and any slot already rewritten earlier in the same
    /// pass."
    ///
    /// The rejection set is therefore read live from `shadowlord_hideouts`,
    /// never from a pre-pass snapshot: that is what makes "a living
    /// Shadowlord never stays in the same town two days running, and no two
    /// living Shadowlords share a town" true. Vanquished `0xFF` slots never
    /// collide with a `1..8` candidate, so they do not constrain the draw,
    /// and with three slots plus one party scene at most four of the eight
    /// ids are ever excluded, so the redraw loop always terminates.
    pub fn reroll_shadowlord_hideouts_excluding(&mut self, current: Option<u8>) -> usize {
        let mut rerolled = 0usize;

        for slot in 0..SHADOWLORD_COUNT {
            if !Self::shadowlord_slot_is_rerollable(self.shadowlord_hideouts[slot]) {
                continue;
            }

            loop {
                let candidate =
                    self.random_range_u8(SHADOWLORD_HIDEOUT_MIN, SHADOWLORD_HIDEOUT_MAX);
                if Some(candidate) == current {
                    continue;
                }
                if self.shadowlord_hideouts.contains(&candidate) {
                    continue;
                }
                self.shadowlord_hideouts[slot] = candidate;
                break;
            }
            rerolled += 1;
        }

        rerolled
    }

    pub fn terrain_encounter_note(
        &self,
        game_dir: Option<&Path>,
        plane: WorldPlane,
        object: ActiveObject,
    ) -> io::Result<String> {
        let hostile_terrain = self.grid[world_cell_index(object.x, object.y)];
        let aboard_ship = matches!(self.player.transport, TransportState::Ship { .. });
        let Some(arena) = outdoor_combat_arena_index(
            object.type_byte,
            hostile_terrain,
            aboard_ship,
            SCENE_OVERWORLD,
        ) else {
            return Ok(format!(
                "no terrain-combat class for active-object type 0x{:02X}",
                object.type_byte
            ));
        };
        let variant = if plane == WorldPlane::Underworld || object.z < 0 {
            " underworld variant"
        } else {
            ""
        };
        let base_class = terrain_combat_base_class(object);
        let base_class_note = base_class
            .map(|stats| format!(", base class {} ({})", stats.name, stats.class))
            .unwrap_or_default();
        let Some(game_dir) = game_dir else {
            return Ok(format!(
                "selected BRIT.CBT arena {arena}{variant}{base_class_note}"
            ));
        };
        if !game_dir.join(BRIT_CBT_FILE).exists() {
            return Ok(format!(
                "selected BRIT.CBT arena {arena}{variant}{base_class_note}"
            ));
        }
        let bank = load_brit_cbt(game_dir)?;
        let record = bank.record(arena).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("BRIT.CBT has no arena record {arena}"),
            )
        })?;
        let setup = terrain_combat_setup_from_record_at_arena(plane, object, arena, record)?;
        let terrain_origin = setup.terrain[0][0];
        let first_placement =
            setup
                .placement_slots
                .first()
                .copied()
                .unwrap_or(CombatPlacementSlot {
                    slot: 0,
                    x: 0,
                    y: 0,
                });
        Ok(format!(
            "loaded BRIT.CBT arena {arena}{variant}{base_class_note} (terrain[0,0]=0x{terrain_origin:02X}, first placement ({}, {}))",
            first_placement.x, first_placement.y
        ))
    }

    pub fn attack_command(&mut self, direction: Option<Direction>) -> MoveOutcome {
        self.attack_command_with_game_dir(direction, None)
            .expect("attack without a game dir cannot load optional arena resources")
    }

    pub fn attack_command_with_game_dir(
        &mut self,
        direction: Option<Direction>,
        game_dir: Option<&Path>,
    ) -> io::Result<MoveOutcome> {
        if let Area::Dungeon { scene, level } = self.area {
            let Some((x, y)) = self.dungeon_forward_target() else {
                // An internal invariant, not a player-facing path: the original
                // prints nothing here, so neither does this. `u5-engine#19`.
                self.message.clear();
                return Ok(MoveOutcome::Blocked);
            };
            let Some((slot, object)) = self.dungeon_attack_monster_record_at(x, y) else {
                // `dungeon-mode.md §10`: "If the active monster is not exactly
                // in that forward cell, the handler prints the stock refusal
                // `What?\n` on the following row and does not launch combat."
                // The engine reported the probed cell and level instead.
                self.push_diagnostic(format!(
                    "attacked forward at ({x}, {y}) on {} level {level}; no target",
                    scene.key()
                ));
                self.message = crate::commands::DUNGEON_ATTACK_NO_TARGET_REFUSAL.to_string();
                return Ok(MoveOutcome::Blocked);
            };
            self.free_active_object_slot(slot);
            self.mark_visibility_dirty();
            // `dungeon-mode.md §10`/§14.1 and `combat.md §4`: the arena is
            // entered from inside the A-Attack dispatch ("combat is a
            // function call"). u5-spec#300 Q10: "No turn is spent by the
            // command itself. On every path, a match or `What?`, A-Attack
            // reports \"no action\" and calls no clock before or after the
            // fight", so the post-action pass does not run for it either; the
            // only minute is the loop head's (§15). The round loop's exit
            // (`apply_combat_round_loop_exit_inner`) therefore spends no turn
            // for this fight.
            // `dungeon-mode.md §14.1`: "Presentation remains caller-owned. The
            // A-Attack path keeps its ordinary Attack command transcript before
            // the matching forward contact", and "Neither caller prints the
            // outdoor conflict banner, and the framer's ambush arm adds no
            // replacement banner." So a launched dungeon attack prints nothing
            // beyond the command echo; the engine printed a sentence naming
            // the tile id, the cell and the level.
            if combat_class_stats(object.aux1).is_some() {
                let note = self.enter_dungeon_active_monster_combat(level, object)?;
                self.set_dungeon_post_combat_applier(DungeonPostCombatApplier::AttackLevelsOnly);
                self.push_diagnostic(format!(
                    "attacked dungeon monster tile {} at ({x}, {y}) on {} level {level}; {note}",
                    object.tile,
                    scene.key()
                ));
                return Ok(MoveOutcome::Used);
            }
            // No arena (an object with no published class): u5-spec#300
            // Q10 makes every A-Attack path "no action" with no clock call.
            self.push_diagnostic(format!(
                "attacked dungeon object tile {} at ({x}, {y}) on {} level {level}; no published combat class",
                object.tile,
                scene.key()
            ));
            return Ok(MoveOutcome::Used);
        }
        // The town on-foot refusal runs before the direction is read.
        if self.town_attack_on_foot_refusal() {
            return Ok(MoveOutcome::Blocked);
        }
        let Some(direction) = direction else {
            // The interactive `A` opens the `Attack-` prompt, so a missing
            // direction can only come from the inline harness form.
            self.message.clear();
            return Ok(MoveOutcome::PromptDeclined);
        };
        let Some((x, y)) = self.adjacent_target(direction) else {
            self.advance_attack_turn_unless_overworld();
            self.message = ATTACK_NOTHING_TO_ATTACK_REFUSAL.to_string();
            return Ok(MoveOutcome::Blocked);
        };
        if let Some((object_slot, object)) = self
            .object_slot_at_current_floor(x, y)
            .map(|(slot, object)| (slot, *object))
        {
            if let Area::World { plane } = self.area {
                if let Some(game_dir) = game_dir {
                    if game_dir.join(BRIT_CBT_FILE).exists()
                        && !is_whirlpool_object(object)
                        && terrain_combat_base_class(object).is_some()
                    {
                        // `overworld.md §5`: A-Attack is step 5's dispatch,
                        // and step 6's per-turn block runs once, after it
                        // returns. `combat.md §4`: "the only time a fight
                        // adds is its round-wrap minutes"; the one turn
                        // advance owed is the round loop's exit
                        // (`apply_combat_round_loop_exit_inner`). Advancing
                        // here as well ran the two-minute block - and its
                        // poison tick - before the arena (playtest H21).
                        let _setup_report = self.enter_terrain_combat_from_world_object(
                            game_dir,
                            plane,
                            object_slot,
                            object,
                        )?;
                        // `commands.md §3`: the attack that enters combat is
                        // still "no action", so the exit owes no outdoor turn
                        // either (p10 R11).
                        self.combat_exit_skips_world_turn = self.combat_active;
                        return Ok(MoveOutcome::Used);
                    }
                }
                // `commands.md §3` (u5-spec#300 Q14): the overworld `A`
                // reports "no action" on every path, so this refusal is free.
                let note = self.terrain_encounter_note(game_dir, plane, object)?;
                // `combat.md §8.2`: the shipped game's whole attack vocabulary
                // is `Attack-`, `Nothing to attack!`, `Attacked!` and
                // `Attacked at entrance!` - there is no line naming a tile,
                // a cell or a slot. This arm could not reach an arena (no
                // `BRIT.CBT`, a whirlpool, or no terrain combat class), so it
                // takes the published refusal (`cleak/u5-engine#18`).
                self.push_diagnostic(format!(
                    "attacked object tile {} at ({x}, {y}) to the {} in slot {object_slot}; {note}",
                    object.tile,
                    direction.name()
                ));
                self.message = ATTACK_NOTHING_TO_ATTACK_REFUSAL.to_string();
                return Ok(MoveOutcome::Used);
            }
            // `town-mode.md §7` step 4 (issue #298 Q40): a town A-Attack that
            // fights in the arena spends nothing of its turn before the
            // arena - "the whole fight ... finish[es] inside the dispatch,
            // before any of this epilogue" - so in town the turn is advanced
            // on each non-arena arm below, and the arena arm leaves it owed
            // to the post-fight epilogue
            // ([`Self::run_town_attack_epilogue_after_combat`]).
            if !matches!(self.area, Area::Town { .. }) {
                self.advance_turn();
            }
            if let Area::Town { scene, floor } = self.area {
                if let Some((npc_index, npc_slot, object_slot, type_byte)) =
                    self.town_attack_target_at(floor, x, y)
                {
                    // `town-mode.md §14`: "The town overlay has a live
                    // NPC-conflict chain, entered both from A-Attack and
                    // from post-action cleanup, that hands the target
                    // NPC's linked active-object slot to the same
                    // terrain-combat entry the overworld uses, so a town
                    // fight is an ordinary arena fight: ordinary town
                    // ground resolves to the cobble arena, and the
                    // scene-keyed town-style override forces the monster
                    // count to one unless the target's class is Guard
                    // (whose stat row carries the sentinel count eight)."
                    // The withdrawn reading kept A-Attack inside town
                    // mode and never called the combat framer.
                    // `town-mode.md §10` (u5-spec#300 Q18): a resident
                    // Shadowlord (`0xFC`) "is therefore a **valid conflict
                    // target**: the attack reaches the NPC-conflict chain
                    // ... which forces arena 10", with no karma loss, no
                    // alarm and no removal mark.
                    let resident_shadowlord = type_byte == SHADOWLORD_ACTOR_TILE;
                    if resident_shadowlord
                        || (town_npc_attack_enters_conflict(type_byte)
                            && !matches!(
                                town_npc_attack_resolution(type_byte),
                                TownNpcAttackResolution::Refused
                            ))
                    {
                        // `combat.md §5`: the encounter's base combat
                        // class "is derived from the creature's own
                        // sprite byte". A town actor's renderer record
                        // carries the drawn person tile rather than its
                        // roster sprite class, so the conflict chain
                        // hands the entry a trigger stamped with the
                        // roster type byte - that is what makes a guard
                        // resolve to class 12 and skip the town-style
                        // single-attacker override.
                        // `combat.md §5`: the placed lead monster "keeps
                        // the triggering object's own tile byte", so the
                        // trigger carries the roster sprite tag in both
                        // sprite bytes rather than the town renderer's
                        // generic person tile.
                        let trigger = ActiveObject {
                            type_byte,
                            tile: type_byte,
                            ..object
                        };
                        if let Some(game_dir) = game_dir {
                            if game_dir.join(BRIT_CBT_FILE).exists()
                                && terrain_combat_base_class(trigger).is_some()
                            {
                                // §14 keeps the alarm sweep on the
                                // A-Attack routing; the death flow's
                                // slot clear, floor reload and
                                // Shadowlord re-install move to the
                                // arena exit.
                                // `0xFC` "triggers neither" karma nor
                                // the alarm (u5-spec#300 Q18).
                                let (pursued, fled) = if resident_shadowlord {
                                    (0, 0)
                                } else {
                                    self.town_alarm_sweep(scene, floor, Some(npc_slot))
                                };
                                self.pending_town_conflict = Some(PendingTownConflict {
                                    scene_byte: scene.byte,
                                    floor,
                                    npc_slot,
                                    type_byte,
                                    awaiting_floor_reload: false,
                                    owes_turn_epilogue: true,
                                });
                                let hostile_terrain = self.grid[y * TOWN_GRID_SIDE + x];
                                let _setup_report = self
                                    .enter_terrain_combat_from_object_in_scene_with_terrain(
                                        game_dir,
                                        WorldPlane::Britannia,
                                        object_slot,
                                        trigger,
                                        scene.byte,
                                        hostile_terrain,
                                    )?;
                                let _ = (pursued, fled);
                                return Ok(MoveOutcome::Used);
                            }
                        }
                    }
                    self.advance_turn();
                    match town_npc_attack_resolution(type_byte) {
                        TownNpcAttackResolution::DeathMask => {
                            self.free_active_object_slot(object_slot);
                            self.npcs.remove(npc_index);
                            self.mark_removed_town_npc_once(scene, npc_slot);
                            let (pursued, fled) =
                                self.town_alarm_sweep(scene, floor, Some(npc_slot));
                            self.mark_visibility_dirty();
                            self.push_diagnostic(format!(
                                "Attacked NPC slot {npc_slot} type 0x{type_byte:02X} at ({x}, {y}) to the {}; target removed from {} floor {floor}; alarm raised ({pursued} pursuing, {fled} fleeing).",
                                direction.name(),
                                scene.key()
                            ));
                            return Ok(MoveOutcome::Used);
                        }
                        TownNpcAttackResolution::AlarmOnly => {
                            let (pursued, fled) =
                                self.town_alarm_sweep(scene, floor, Some(npc_slot));
                            self.push_diagnostic(format!(
                                "Attacked NPC slot {npc_slot} type 0x{type_byte:02X} at ({x}, {y}) to the {}; alarm raised ({pursued} pursuing, {fled} fleeing).",
                                direction.name()
                            ));
                            return Ok(MoveOutcome::Used);
                        }
                        TownNpcAttackResolution::Refused => {
                            self.push_diagnostic(format!(
                                "Attacked NPC slot {npc_slot} type 0x{type_byte:02X} at ({x}, {y}) to the {}; no attackable town NPC.",
                                direction.name()
                            ));
                            // `town-mode.md §7`: "Invalid targets produce the
                            // ordinary failure text" - `combat.md §8.2`'s
                            // `Nothing to attack!`. This arm used to print
                            // nothing at all, so `A` at a resident Shadowlord
                            // read only `Attack-North` (playtest K08).
                            self.message = ATTACK_NOTHING_TO_ATTACK_REFUSAL.to_string();
                            return Ok(MoveOutcome::Blocked);
                        }
                    }
                }
                self.advance_turn();
                self.push_diagnostic(format!(
                    "Attacked object tile {} at ({x}, {y}) to the {}; no attackable town NPC.",
                    object.tile,
                    direction.name()
                ));
                // `town-mode.md §7`: the ordinary failure text (playtest K08).
                self.message = ATTACK_NOTHING_TO_ATTACK_REFUSAL.to_string();
                return Ok(MoveOutcome::Blocked);
            }
            self.push_diagnostic(format!(
                "attacked object tile {} at ({x}, {y}) to the {} in slot {object_slot}; no published combat class",
                object.tile,
                direction.name()
            ));
            self.message = ATTACK_NOTHING_TO_ATTACK_REFUSAL.to_string();
            return Ok(MoveOutcome::Used);
        }

        self.advance_attack_turn_unless_overworld();
        // Measured: `A` into an empty adjacent cell prints `Attack-North`
        // and then this refusal - not a composed line naming the cell.
        self.message = ATTACK_NOTHING_TO_ATTACK_REFUSAL.to_string();
        Ok(MoveOutcome::Blocked)
    }

    /// `commands.md §3`, the forwarded-route table: "`A` overworld | Always
    /// \"no action\", including the successful attack that enters combat"
    /// - "Zero skips the whole per-turn epilogue" - while "`A` town |
    /// \"Acted\" ... The nothing-to-attack message still counts as acted."
    /// So a refused overworld attack spends no outdoor turn (p10 R01); the
    /// town refusal keeps its minute.
    fn advance_attack_turn_unless_overworld(&mut self) {
        if !matches!(self.area, Area::World { .. }) {
            self.advance_turn();
        }
    }

    pub fn dungeon_forward_target(&self) -> Option<(usize, usize)> {
        let Area::Dungeon { .. } = self.area else {
            return None;
        };
        let direction = if self.player.facing.is_cardinal() {
            self.player.facing
        } else {
            return None;
        };
        let (dx, dy) = direction.delta();
        let x = (self.player.x as isize + dx).rem_euclid(DUNGEON_SIDE as isize) as usize;
        let y = (self.player.y as isize + dy).rem_euclid(DUNGEON_SIDE as isize) as usize;
        Some((x, y))
    }

    /// [`Self::dungeon_active_monster`] when it stands on `(x, y)`.
    pub fn dungeon_active_monster_at(&self, x: usize, y: usize) -> Option<(usize, ActiveObject)> {
        self.dungeon_active_monster()
            .filter(|(_, object)| object.x == x && object.y == y)
    }

    /// The A-Attack lookup: the single dungeon monster record when its
    /// coordinate is `(x, y)`, **whatever its class byte**.
    /// `dungeon-mode.md §4.1` (u5-spec#300 Q24): "A-Attack does not test the
    /// inactive marker at all; it relies on those off-grid coordinates never
    /// matching a cell", and `§10` "compares that coordinate with the single
    /// active dungeon monster record". The painter, step, contact and
    /// blocking paths keep the `0xFF` test (playtest S27).
    pub fn dungeon_attack_monster_record_at(
        &self,
        x: usize,
        y: usize,
    ) -> Option<(usize, ActiveObject)> {
        let Area::Dungeon { .. } = self.area else {
            return None;
        };
        self.active_objects
            .get(DUNGEON_ACTIVE_MONSTER_SLOT)
            .copied()
            .filter(|object| object.x == x && object.y == y)
            .map(|object| (DUNGEON_ACTIVE_MONSTER_SLOT, object))
    }

    /// `town-mode.md §14`: the first half of the NPC-conflict chain's
    /// exit - "On exit the town chain clears the NPC slot". The
    /// removal-mask policy of §4 still decides whether the cleared slot
    /// is recorded as permanently gone: "killing a townsperson or a
    /// named character records that slot as permanently removed in the
    /// per-scene removal mask (Section 4), while killing a guard or a
    /// creature records nothing and that slot is placed again on the
    /// next entry."
    pub fn clear_town_conflict_npc_slot(&mut self, scene: Scene, npc_slot: usize, type_byte: u8) {
        if let Some(npc_index) = self.npcs.iter().position(|npc| npc.slot == npc_slot) {
            if let Some(object_slot) = self.npcs[npc_index].active_object {
                self.free_active_object_slot(object_slot);
            }
            self.npcs.remove(npc_index);
        }
        if town_npc_removal_recorded(type_byte) {
            self.mark_removed_town_npc_once(scene, npc_slot);
        }
        self.mark_visibility_dirty();
    }

    /// `town-mode.md §14`: the rest of the NPC-conflict chain's exit -
    /// the chain "reloads the town map, and re-runs the Shadowlord
    /// install pass of Section 13 (which, in a hideout town whose
    /// Shadowlord is still standing in the active-object table, is
    /// rejected by the one-at-a-time check and does nothing)". Both
    /// halves need a game directory, so they are drained at the input
    /// boundary once the arena frame has been restored.
    pub fn drain_pending_town_conflict(&mut self, game_dir: &Path) -> io::Result<bool> {
        if self.combat_active {
            return Ok(false);
        }
        let Some(pending) = self.pending_town_conflict else {
            return Ok(false);
        };
        if !pending.awaiting_floor_reload {
            return Ok(false);
        }
        self.pending_town_conflict = None;
        let Area::Town { scene, floor } = self.area else {
            return Ok(false);
        };
        if scene.byte != pending.scene_byte {
            return Ok(false);
        }
        self.reload_town_floor(game_dir, scene, floor)?;
        // `encounters.md §7`: the chain "does not re-place the player:
        // the player's position comes from the world-state globals
        // throughout."
        self.install_shadowlord_entry_encounter();
        Ok(true)
    }

    pub fn town_attack_target_at(
        &self,
        floor: i8,
        x: usize,
        y: usize,
    ) -> Option<(usize, usize, usize, u8)> {
        if floor < 0 {
            return None;
        }
        let floor = floor as u8;
        self.npcs.iter().enumerate().find_map(|(index, npc)| {
            if npc.x != x || npc.y != y || npc.z != floor {
                return None;
            }
            let object_slot = npc.active_object.or_else(|| {
                self.active_objects
                    .iter()
                    .copied()
                    .enumerate()
                    .skip(1)
                    .find_map(|(slot, object)| {
                        active_object_matches_runtime_npc(object, npc, floor).then_some(slot)
                    })
            })?;
            Some((index, npc.slot, object_slot, npc.type_byte))
        })
    }

    pub fn adjacent_target(&self, direction: Direction) -> Option<(usize, usize)> {
        let (dx, dy) = direction.delta();
        match self.area {
            Area::World { .. } => {
                let x = (self.player.x as isize + dx).rem_euclid(WORLD_SIDE as isize) as usize;
                let y = (self.player.y as isize + dy).rem_euclid(WORLD_SIDE as isize) as usize;
                Some((x, y))
            }
            Area::Town { .. } => {
                let x = self.player.x as isize + dx;
                let y = self.player.y as isize + dy;
                if (0..32).contains(&x) && (0..32).contains(&y) {
                    Some((x as usize, y as usize))
                } else {
                    None
                }
            }
            Area::Dungeon { .. } => {
                let x = self.player.x as isize + dx;
                let y = self.player.y as isize + dy;
                if (0..DUNGEON_SIDE as isize).contains(&x)
                    && (0..DUNGEON_SIDE as isize).contains(&y)
                {
                    Some((x as usize, y as usize))
                } else {
                    None
                }
            }
        }
    }

    pub fn fire_command(
        &mut self,
        direction: Option<Direction>,
        game_dir: &Path,
    ) -> io::Result<MoveOutcome> {
        match self.area {
            Area::World { .. } => Ok(self.fire_ship_broadside(direction)),
            Area::Town { scene, floor } => self.fire_town_source(game_dir, scene, floor),
            Area::Dungeon { .. } => {
                self.message = "What?".to_string();
                Ok(MoveOutcome::Blocked)
            }
        }
    }

    pub fn fire_town_source(
        &mut self,
        game_dir: &Path,
        scene: Scene,
        floor: i8,
    ) -> io::Result<MoveOutcome> {
        let entries = load_town_fire_source_entries(game_dir)?;
        self.tick_door_tracker();
        let Some(source) = self.town_fire_source(scene, floor, entries.as_deref()) else {
            self.message = "What?".to_string();
            return Ok(MoveOutcome::Blocked);
        };

        let target = self.town_fire_target(source);
        match target {
            TownFireTarget::Object { slot, object } => {
                self.free_active_object_slot(slot);
                self.mark_visibility_dirty();
                let moral_delta = self.decrease_moral_standing(TOWN_CANNON_HIT_KARMA_DEBIT);
                self.advance_turn_without_door_tick();
                self.diagnostics.push(format!(
                    "town cannon at ({}, {}) fired {} and hit object tile {} at ({}, {}); \
                     target removed, moral standing -{moral_delta}",
                    source.x,
                    source.y,
                    source.direction.name(),
                    object.tile,
                    object.x,
                    object.y
                ));
                self.message = crate::commands::CANNON_BOOOM_LINE.to_string();
            }
            TownFireTarget::Door { x, y, tile } => {
                self.grid[y * 32 + x] = TOWN_DOOR_CLEARED_TILE;
                self.record_open_town_door(scene, floor, x, y);
                self.forget_revealed_town_secret_door(scene, floor, x, y);
                self.door_tracker = None;
                self.door_tracker_closed = false;
                self.mark_visibility_dirty();
                self.advance_turn_without_door_tick();
                self.diagnostics.push(format!(
                    "town cannon at ({}, {}) fired {} and destroyed door tile {tile} at ({x}, {y})",
                    source.x,
                    source.y,
                    source.direction.name(),
                ));
                self.message = format!(
                    "{} {}",
                    crate::commands::CANNON_BOOOM_LINE,
                    crate::commands::CANNON_DOOR_DESTROYED_LINE
                );
            }
            TownFireTarget::Wall { x, y, tile } => {
                self.advance_turn_without_door_tick();
                self.diagnostics.push(format!(
                    "town cannon at ({}, {}) fired {} and hit blocking tile {tile} at ({x}, {y})",
                    source.x,
                    source.y,
                    source.direction.name(),
                ));
                self.message = crate::commands::CANNON_BOOOM_LINE.to_string();
            }
            TownFireTarget::None => {
                self.advance_turn_without_door_tick();
                self.diagnostics.push(format!(
                    "town cannon at ({}, {}) fired {} with no target in range",
                    source.x,
                    source.y,
                    source.direction.name()
                ));
                self.message = crate::commands::CANNON_BOOOM_LINE.to_string();
            }
        }
        Ok(MoveOutcome::Fired)
    }

    pub fn town_fire_source(
        &self,
        scene: Scene,
        floor: i8,
        sidecar_entries: Option<&[TownFireSourceEntry]>,
    ) -> Option<TownFireSourceEntry> {
        if let Some(entry) = sidecar_entries.and_then(|entries| {
            entries
                .iter()
                .find(|entry| {
                    let entry = **entry;
                    entry.scene == scene
                        && entry.floor == floor
                        && town_fire_source_is_adjacent(entry, self.player.x, self.player.y)
                        && town_fire_source_tile_matches(entry, self.grid[entry.y * 32 + entry.x])
                })
                .copied()
        }) {
            return Some(entry);
        }

        self.adjacent_town_cannon_fire_source(scene, floor)
    }

    pub fn adjacent_town_cannon_fire_source(
        &self,
        scene: Scene,
        floor: i8,
    ) -> Option<TownFireSourceEntry> {
        for dy in -1isize..=1 {
            for dx in -1isize..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let x = self.player.x as isize + dx;
                let y = self.player.y as isize + dy;
                if !(0..32).contains(&x) || !(0..32).contains(&y) {
                    continue;
                }
                let x = x as usize;
                let y = y as usize;
                let tile = self.grid[y * 32 + x];
                let Some(direction) = town_cannon_tile_fire_direction(tile) else {
                    continue;
                };
                return Some(TownFireSourceEntry {
                    scene,
                    floor,
                    x,
                    y,
                    direction,
                    expected_tile: Some(tile),
                });
            }
        }
        None
    }

    pub fn town_fire_target(&self, source: TownFireSourceEntry) -> TownFireTarget {
        let (dx, dy) = source.direction.delta();
        for distance in 1..=(TOWN_CANNON_RANGE_CELLS as isize) {
            let x = source.x as isize + dx * distance;
            let y = source.y as isize + dy * distance;
            if !(0..32).contains(&x) || !(0..32).contains(&y) {
                break;
            }
            let x = x as usize;
            let y = y as usize;
            if let Some((slot, object)) = self
                .active_objects
                .iter()
                .enumerate()
                .skip(1)
                .find(|(_, object)| self.object_occupies(**object, x, y))
                .map(|(slot, object)| (slot, *object))
            {
                return TownFireTarget::Object { slot, object };
            }

            let tile = self.grid[y * 32 + x];
            if town_command_door_tile(tile) {
                return TownFireTarget::Door { x, y, tile };
            }
            if surface_tile_blocks_projectile(tile) {
                return TownFireTarget::Wall { x, y, tile };
            }
        }
        TownFireTarget::None
    }

    /// `vehicles.md §7` overworld F-Fire ship-broadside helper.
    ///
    /// Step 2's shared pre-action gate is the shared direction prompt
    /// (`doors-and-z-transitions.md §9`, `RETRACTIONS.md` R470): a call
    /// without a direction opens that prompt, and its cancel arm aborts
    /// without a shot. The helper prints no narration of its own after the
    /// direction echo; steps 5 and 7 are presentation only (an impact on a
    /// hit, the projectile flown to the endpoint on a miss), so a shot
    /// requests a viewport repaint and records the traced endpoint as a
    /// diagnostic rather than writing message text.
    pub fn fire_ship_broadside(&mut self, direction: Option<Direction>) -> MoveOutcome {
        if !matches!(self.area, Area::World { .. }) {
            self.message = "What?".to_string();
            return MoveOutcome::Blocked;
        }
        if !matches!(self.player.transport, TransportState::Ship { .. }) {
            self.message = "What?".to_string();
            return MoveOutcome::Blocked;
        }
        let Some(direction) = direction else {
            return self.start_fire_direction_prompt();
        };
        // `vehicles.md §7` step 3 gives the bow/stern refusal only its text,
        // and `commands.md §3` does not forward F's handler value, so the
        // refusal reports "acted" and costs the outdoor turn like every other
        // F outcome (playtest K18).
        let broadside = self
            .player
            .facing
            .cardinal_facing_index()
            .zip(direction.cardinal_facing_index())
            .is_some_and(|(ship_facing, fire_direction)| {
                ship_broadside_direction_accepted(ship_facing, fire_direction)
            });
        if !broadside {
            self.advance_turn();
            self.message = "Fire broadsides only!".to_string();
            return MoveOutcome::Blocked;
        }

        let hit = self
            .ship_broadside_target_slot(direction)
            .map(|slot| (slot, self.active_objects[slot]));
        if let Some((slot, object)) = hit {
            let damage = self.ship_broadside_damage_roll();
            if let Some(remaining) = ship_broadside_apply_damage(object.aux1, damage) {
                if let Some(target) = self.active_objects.get_mut(slot) {
                    target.aux1 = remaining;
                }
            } else {
                self.free_active_object_slot(slot);
            }
            // §7 step 5: the impact is animated at the struck object.
            self.diagnostics.push(format!(
                "ship broadside fired {} and hit object tile {} at ({}, {}) for {damage}",
                direction.name(),
                object.tile,
                object.x,
                object.y
            ));
        } else {
            // §7 step 7: with no target the projectile still flies to the
            // end of the up-to-three-cell line.
            let (dx, dy) = direction.delta();
            let reach = SHIP_BROADSIDE_RANGE_CELLS as isize;
            let x = (self.player.x as isize + dx * reach).rem_euclid(WORLD_SIDE as isize);
            let y = (self.player.y as isize + dy * reach).rem_euclid(WORLD_SIDE as isize);
            self.diagnostics.push(format!(
                "ship broadside fired {} with no target; projectile ends at ({x}, {y})",
                direction.name()
            ));
        }
        self.mark_visibility_dirty();
        self.advance_turn();
        MoveOutcome::Fired
    }

    pub fn ship_broadside_target_slot(&self, direction: Direction) -> Option<usize> {
        let (dx, dy) = direction.delta();
        for distance in 1..=SHIP_BROADSIDE_RANGE_CELLS as isize {
            let x = (self.player.x as isize + dx * distance).rem_euclid(WORLD_SIDE as isize);
            let y = (self.player.y as isize + dy * distance).rem_euclid(WORLD_SIDE as isize);
            let x = x as usize;
            let y = y as usize;
            if let Some((slot, _)) =
                self.active_objects
                    .iter()
                    .enumerate()
                    .skip(1)
                    .find(|(_, object)| {
                        self.object_occupies(**object, x, y) && !is_whirlpool_object(**object)
                    })
            {
                return Some(slot);
            }
        }
        None
    }

    pub fn ship_broadside_damage_roll(&mut self) -> u8 {
        self.random_range_u8(SHIP_BROADSIDE_DAMAGE_MIN, SHIP_BROADSIDE_DAMAGE_MAX)
    }

    pub fn decrease_moral_standing(&mut self, amount: u8) -> u8 {
        let before = self.moral_standing;
        self.moral_standing = self.moral_standing.saturating_sub(amount);
        before - self.moral_standing
    }

    pub fn push_facing_with_game_dir(&mut self, game_dir: &Path) -> io::Result<MoveOutcome> {
        self.push_direction_with_game_dir(self.player.facing, game_dir)
    }

    pub fn push_direction_with_game_dir(
        &mut self,
        direction: Direction,
        game_dir: &Path,
    ) -> io::Result<MoveOutcome> {
        if matches!(self.area, Area::Dungeon { .. }) {
            self.message = PUSH_NOT_HERE_REFUSAL.to_string();
            return Ok(MoveOutcome::Blocked);
        }
        self.tick_door_tracker();
        let outcome = self.push_direction_after_cleanup_with_game_dir(direction, game_dir)?;
        self.prepend_push_direction_result(direction);
        Ok(outcome)
    }

    pub(crate) fn push_direction_after_cleanup_with_game_dir(
        &mut self,
        direction: Direction,
        game_dir: &Path,
    ) -> io::Result<MoveOutcome> {
        match self.area {
            Area::Town { scene, floor } => {
                self.push_town_direction_after_cleanup(game_dir, scene, floor, direction)
            }
            Area::Dungeon { .. } => unreachable!("dungeon Push bypasses direction handling"),
            Area::World { .. } => Ok(self.push_world_direction(direction)),
        }
    }

    pub(crate) fn prepend_push_direction_result(&mut self, direction: Direction) {
        let result = std::mem::take(&mut self.message);
        self.message = if result.is_empty() {
            direction.name().to_string()
        } else {
            format!("{}\n{result}", direction.name())
        };
    }

    pub fn push_world_direction(&mut self, direction: Direction) -> MoveOutcome {
        if !direction.is_cardinal() {
            // An internal invariant, not a player-facing path: the original
            // prints nothing here, so neither does this. `u5-engine#19`.
            self.message.clear();
            return MoveOutcome::Blocked;
        }
        let (tx, ty) = self.world_push_coordinate(self.player.x, self.player.y, direction, 1);
        let (px, py) = self.world_push_coordinate(self.player.x, self.player.y, direction, 2);
        let target_idx = world_cell_index(tx, ty);
        let target_tile = self.grid[target_idx];

        if self.object_slot_at_current_floor(tx, ty).is_some() {
            self.advance_turn_without_door_tick();
            self.message = PUSH_WONT_BUDGE_EMPHATIC.to_string();
            return MoveOutcome::Blocked;
        }

        let Some(family) = pushable_tile_family(target_tile) else {
            // `commands.md §3`: P-Push does not forward the handler's
            // refusal status. Outside dungeons the resident dispatcher keeps
            // its default acted result, so even a source miss consumes the
            // ordinary world action.
            self.advance_turn();
            self.message = PUSH_WONT_BUDGE_EMPHATIC.to_string();
            return MoveOutcome::Blocked;
        };

        self.push_world_static_family(direction, tx, ty, px, py, family)
    }

    pub fn push_combat_actor_direction(
        &mut self,
        actor_slot: usize,
        direction: Direction,
    ) -> MoveOutcome {
        self.tick_door_tracker();
        let outcome = self.push_combat_actor_direction_after_cleanup(actor_slot, direction);
        self.prepend_push_direction_result(direction);
        outcome
    }

    pub(crate) fn push_combat_actor_direction_after_cleanup(
        &mut self,
        actor_slot: usize,
        direction: Direction,
    ) -> MoveOutcome {
        if !self.combat_active || actor_slot >= COMBAT_PARTY_ACTOR_SLOTS {
            self.message.clear();
            return MoveOutcome::Blocked;
        }
        if !direction.is_cardinal() {
            self.message.clear();
            return MoveOutcome::Blocked;
        }
        let Some(actor) = self.combat_actors.get(actor_slot).copied() else {
            self.message.clear();
            return MoveOutcome::Blocked;
        };
        if !combat_actor_is_active_not_dead(actor) {
            self.message.clear();
            return MoveOutcome::Blocked;
        }

        let (dx, dy) = direction.delta();
        let sx = actor.x as isize + dx;
        let sy = actor.y as isize + dy;
        let dx2 = sx + dx;
        let dy2 = sy + dy;
        if apply_combat_ambush_reveal_records(
            &mut self.combat_ambush_reveals,
            &mut self.combat_terrain,
            sx as u8,
            sy as u8,
        )
        .is_some()
        {
            // `commands.md §8.2`: a pre-placed ambush/camp reveal marker
            // consumes the action before the ordinary source predicate and
            // has no result continuation after the direction echo.
            self.mark_visibility_dirty();
            self.message.clear();
            return MoveOutcome::Pushed;
        }
        if !combat_arena_coordinate_in_bounds(sx as i16, sy as i16)
            || !combat_arena_coordinate_in_bounds(dx2 as i16, dy2 as i16)
        {
            // The finite clean arena has no exposed backing bytes. Its
            // default off-grid sample is zero/non-pushable; tests that model
            // another backing byte must provide an in-range fixture cell.
            self.message = PUSH_WONT_BUDGE_EMPHATIC.to_string();
            return MoveOutcome::Blocked;
        }
        let sx = sx as usize;
        let sy = sy as usize;
        let dx2 = dx2 as usize;
        let dy2 = dy2 as usize;

        if self
            .combat_actor_slot_at(sx as u8, sy as u8, actor_slot)
            .is_some()
        {
            self.message = PUSH_WONT_BUDGE_EMPHATIC.to_string();
            return MoveOutcome::Blocked;
        }

        if self
            .combat_loose_object_slot_at(sx, sy, actor.active_object_slot as usize)
            .is_some()
        {
            self.message = PUSH_WONT_BUDGE_EMPHATIC.to_string();
            return MoveOutcome::Blocked;
        }

        let source_tile = self.combat_terrain[sy][sx];
        let Some(family) = pushable_tile_family(source_tile) else {
            self.message = PUSH_WONT_BUDGE_EMPHATIC.to_string();
            return MoveOutcome::Blocked;
        };

        self.push_combat_static_family(actor_slot, direction, sx, sy, dx2, dy2, family)
    }

    fn push_combat_static_family(
        &mut self,
        actor_slot: usize,
        direction: Direction,
        sx: usize,
        sy: usize,
        dx2: usize,
        dy2: usize,
        family: PushableTileFamily,
    ) -> MoveOutcome {
        let actor = self.combat_actors[actor_slot];
        let source_tile = self.combat_terrain[sy][sx];
        let stamp = family.floor_stamp();
        if self.combat_cell_clear_for_push(dx2, dy2) && self.combat_terrain[dy2][dx2] == stamp {
            self.combat_terrain[sy][sx] = stamp;
            self.combat_terrain[dy2][dx2] = pushable_oriented_tile(source_tile, direction);
            self.finish_combat_push(actor_slot, sx, sy);
            self.message = PUSHED_SUCCESS.to_string();
            return MoveOutcome::Pushed;
        }

        if self.combat_terrain[actor.y as usize][actor.x as usize] == stamp {
            let pull_direction = direction.opposite_cardinal().unwrap_or(direction);
            self.combat_terrain[actor.y as usize][actor.x as usize] =
                pushable_oriented_tile(source_tile, pull_direction);
            self.combat_terrain[sy][sx] = stamp;
            self.finish_combat_push(actor_slot, sx, sy);
            self.message = PULLED_SUCCESS.to_string();
            return MoveOutcome::Pushed;
        }

        self.message = PUSH_WONT_BUDGE_SHORT.to_string();
        MoveOutcome::Blocked
    }

    fn finish_combat_push(&mut self, actor_slot: usize, x: usize, y: usize) {
        let _ = commit_combat_actor_linked_position(
            &mut self.combat_actors[actor_slot],
            &mut self.active_objects,
            x as u8,
            y as u8,
        );
        self.mark_visibility_dirty();
    }

    pub fn combat_actor_slot_at(&self, x: u8, y: u8, except_slot: usize) -> Option<usize> {
        self.combat_actors
            .iter()
            .copied()
            .enumerate()
            .find_map(|(slot, actor)| {
                (slot != except_slot && combat_actor_occupies_arena_cell(actor, x, y))
                    .then_some(slot)
            })
    }

    pub fn combat_loose_object_slot_at(
        &self,
        x: usize,
        y: usize,
        actor_object_slot: usize,
    ) -> Option<(usize, ActiveObject)> {
        self.active_objects
            .iter()
            .copied()
            .enumerate()
            .find_map(|(slot, object)| {
                if slot == actor_object_slot || object.is_empty() || object.x != x || object.y != y
                {
                    return None;
                }
                let linked_to_live_actor = self.combat_actors.iter().copied().any(|actor| {
                    combat_actor_is_active_not_dead(actor)
                        && actor.active_object_slot as usize == slot
                });
                (!linked_to_live_actor).then_some((slot, object))
            })
    }

    fn combat_cell_clear_for_push(&self, x: usize, y: usize) -> bool {
        self.combat_actor_slot_at(x as u8, y as u8, usize::MAX)
            .is_none()
            && self
                .active_objects
                .iter()
                .copied()
                .all(|object| object.is_empty() || object.x != x || object.y != y)
    }

    fn world_push_coordinate(
        &self,
        x: usize,
        y: usize,
        direction: Direction,
        distance: isize,
    ) -> (usize, usize) {
        let (dx, dy) = direction.delta();
        (
            (x as isize + dx * distance).rem_euclid(WORLD_SIDE as isize) as usize,
            (y as isize + dy * distance).rem_euclid(WORLD_SIDE as isize) as usize,
        )
    }

    fn push_world_static_family(
        &mut self,
        direction: Direction,
        tx: usize,
        ty: usize,
        px: usize,
        py: usize,
        family: PushableTileFamily,
    ) -> MoveOutcome {
        let target_idx = world_cell_index(tx, ty);
        let dest_idx = world_cell_index(px, py);
        let player_idx = world_cell_index(self.player.x, self.player.y);
        let target_tile = self.grid[target_idx];
        let stamp = family.floor_stamp();
        if self.blocking_object_at(px, py).is_none() && self.grid[dest_idx] == stamp {
            self.grid[target_idx] = stamp;
            self.grid[dest_idx] = pushable_oriented_tile(target_tile, direction);
            self.finish_world_push(tx, ty);
            self.message = PUSHED_SUCCESS.to_string();
            return MoveOutcome::Pushed;
        }

        if self.grid[player_idx] == stamp {
            let pull_direction = direction.opposite_cardinal().unwrap_or(direction);
            self.grid[player_idx] = pushable_oriented_tile(target_tile, pull_direction);
            self.grid[target_idx] = stamp;
            self.finish_world_push(tx, ty);
            self.message = PULLED_SUCCESS.to_string();
            return MoveOutcome::Pushed;
        }

        self.advance_turn();
        self.message = PUSH_WONT_BUDGE_SHORT.to_string();
        MoveOutcome::Blocked
    }

    fn finish_world_push(&mut self, tx: usize, ty: usize) {
        self.player.x = tx;
        self.player.y = ty;
        self.sync_player_object();
        let _ = self.refresh_world_live_chunks_for_current_area();
        self.mark_visibility_dirty();
        self.advance_turn();
    }

    pub fn push_town_facing(
        &mut self,
        game_dir: &Path,
        scene: Scene,
        floor: i8,
    ) -> io::Result<MoveOutcome> {
        self.push_town_direction(game_dir, scene, floor, self.player.facing)
    }

    pub fn push_town_direction(
        &mut self,
        game_dir: &Path,
        scene: Scene,
        floor: i8,
        direction: Direction,
    ) -> io::Result<MoveOutcome> {
        self.tick_door_tracker();
        let outcome = self.push_town_direction_after_cleanup(game_dir, scene, floor, direction)?;
        self.prepend_push_direction_result(direction);
        Ok(outcome)
    }

    pub(crate) fn push_town_direction_after_cleanup(
        &mut self,
        game_dir: &Path,
        scene: Scene,
        floor: i8,
        direction: Direction,
    ) -> io::Result<MoveOutcome> {
        if !direction.is_cardinal() {
            self.message.clear();
            return Ok(MoveOutcome::Blocked);
        }
        let (dx, dy) = direction.delta();
        let tx = self.player.x as isize + dx;
        let ty = self.player.y as isize + dy;
        let px = tx + dx;
        let py = ty + dy;
        // `commands.md §8.2`: an out-of-grid location tile read aliases
        // the southeast cell, while the true coordinate cannot match an
        // ordinary active-object record.
        let source_in_bounds = (0..32).contains(&tx) && (0..32).contains(&ty);
        let far_in_bounds = (0..32).contains(&px) && (0..32).contains(&py);
        let (tx, ty) = if source_in_bounds {
            (tx as usize, ty as usize)
        } else {
            (31, 31)
        };
        let (px, py) = if far_in_bounds {
            (px as usize, py as usize)
        } else {
            (31, 31)
        };
        let target_idx = ty * 32 + tx;
        let target_tile = self.grid[target_idx];
        let entries = load_town_pushable_entries(game_dir)?;
        let sidecar_pushable = entries.as_ref().is_some_and(|entries| {
            entries
                .iter()
                .any(|entry| town_pushable_matches(*entry, scene, floor, tx, ty, target_tile))
        });

        if source_in_bounds && self.object_slot_at_current_floor(tx, ty).is_some() {
            self.advance_turn_without_door_tick();
            self.message = PUSH_WONT_BUDGE_EMPHATIC.to_string();
            return Ok(MoveOutcome::Blocked);
        }

        let Some(family) = pushable_tile_family(target_tile) else {
            if sidecar_pushable {
                return Ok(self.push_town_sidecar_tile(scene, floor, direction, tx, ty, px, py));
            }
            self.advance_turn_without_door_tick();
            self.message = PUSH_WONT_BUDGE_EMPHATIC.to_string();
            return Ok(MoveOutcome::Blocked);
        };

        Ok(self.push_town_static_family(scene, floor, direction, tx, ty, px, py, family))
    }

    fn push_town_sidecar_tile(
        &mut self,
        scene: Scene,
        floor: i8,
        _direction: Direction,
        tx: usize,
        ty: usize,
        px: usize,
        py: usize,
    ) -> MoveOutcome {
        if self.blocking_object_at(px, py).is_some() {
            self.advance_turn_without_door_tick();
            self.message = PUSH_WONT_BUDGE_SHORT.to_string();
            return MoveOutcome::Blocked;
        }
        let target_idx = ty * 32 + tx;
        let dest_idx = py * 32 + px;
        let target_tile = self.grid[target_idx];
        let dest_tile = self.grid[dest_idx];
        if !self.tile_walkable(dest_tile) {
            self.advance_turn_without_door_tick();
            self.message = PUSH_WONT_BUDGE_SHORT.to_string();
            return MoveOutcome::Blocked;
        }

        self.grid[target_idx] = dest_tile;
        self.grid[dest_idx] = target_tile;
        self.finish_town_push(scene, floor, tx, ty, px, py);
        self.message = PUSHED_SUCCESS.to_string();
        MoveOutcome::Pushed
    }

    fn push_town_static_family(
        &mut self,
        scene: Scene,
        floor: i8,
        direction: Direction,
        tx: usize,
        ty: usize,
        px: usize,
        py: usize,
        family: PushableTileFamily,
    ) -> MoveOutcome {
        let target_idx = ty * 32 + tx;
        let dest_idx = py * 32 + px;
        let player_idx = self.player.y * 32 + self.player.x;
        let target_tile = self.grid[target_idx];
        let stamp = family.floor_stamp();
        if self.blocking_object_at(px, py).is_none() && self.grid[dest_idx] == stamp {
            self.grid[target_idx] = stamp;
            self.grid[dest_idx] = pushable_oriented_tile(target_tile, direction);
            self.finish_town_push(scene, floor, tx, ty, px, py);
            self.message = PUSHED_SUCCESS.to_string();
            return MoveOutcome::Pushed;
        }

        if self.grid[player_idx] == stamp {
            let pull_direction = direction.opposite_cardinal().unwrap_or(direction);
            let old_player_x = self.player.x;
            let old_player_y = self.player.y;
            self.grid[player_idx] = pushable_oriented_tile(target_tile, pull_direction);
            self.grid[target_idx] = stamp;
            self.finish_town_push(scene, floor, tx, ty, old_player_x, old_player_y);
            self.message = PULLED_SUCCESS.to_string();
            return MoveOutcome::Pushed;
        }

        self.advance_turn_without_door_tick();
        self.message = PUSH_WONT_BUDGE_SHORT.to_string();
        MoveOutcome::Blocked
    }

    fn finish_town_push(
        &mut self,
        scene: Scene,
        floor: i8,
        tx: usize,
        ty: usize,
        px: usize,
        py: usize,
    ) {
        self.forget_open_town_door(scene, floor, tx, ty);
        self.forget_open_town_door(scene, floor, px, py);
        self.forget_revealed_town_secret_door(scene, floor, tx, ty);
        self.forget_revealed_town_secret_door(scene, floor, px, py);
        // `doors-and-z-transitions.md §5`: the auto-close block is cancelled
        // only by a second Open, combat, the town reload and the cannon's
        // destroyed door. A push or pull through the tracked doorway leaves
        // it running, so the countdown still writes the door back (playtest
        // Q04).
        self.player.x = tx;
        self.player.y = ty;
        self.sync_player_object();
        self.mark_visibility_dirty();
        self.advance_turn_without_door_tick();
    }

    #[cfg(test)]
    pub fn pass_turn(&mut self) -> MoveOutcome {
        self.pass_turn_with_game_dir(None)
            .expect("sidecar-free pass cannot fail")
    }

    pub fn pass_turn_with_game_dir(&mut self, game_dir: Option<&Path>) -> io::Result<MoveOutcome> {
        let turn_before = self.turn;
        self.advance_turn();
        // `weather.md §5.1`, clear three: "The Pass command | Only while the
        // outdoor scene is current and the cache is non-zero; it prints the
        // stalled-sailing line first, then clears." The condition is the
        // cache, not the engine's own stall flag: a Pass taken on a released
        // pass still had a heading cached and still clears it.
        if matches!(self.area, Area::World { .. }) && self.sail_cached_direction.is_some() {
            self.sail_cached_direction = None;
            // `weather.md §5`: "That Pass adds no ordinary `Pass` echo", and
            // `vehicles.md §6`: it "does not also print `Pass`". The stalled
            // line takes the echo's place on the command row rather than
            // following it (playtest E26).
            if self.pending_command_echo.is_some() {
                self.replace_command_echo(CommandEcho {
                    text: SHIP_SAIL_STALLED_LINE,
                    join: CommandEchoJoin::Complete,
                });
                self.message = String::new();
            } else {
                self.message = SHIP_SAIL_STALLED_LINE.to_string();
            }
        } else {
            // `commands.md §8.1` row B: `Pass` completes its own echo and
            // "no result line follows"; `text-output.md §10.3` lists the
            // `Pass` echo as complete in itself. The observed original
            // prints the echo and nothing beneath it, so the slot is left
            // empty for whatever the turn epilogue produces.
            self.message = String::new();
        }
        if let Some(game_dir) = game_dir {
            if let Some(outcome) =
                self.apply_top_down_post_turn_effects_after_turn(turn_before, game_dir)?
            {
                return Ok(outcome);
            }
        } else {
            // Test-only/sidecar-free callers have no town post-action I/O
            // path to enter. They still finish the deferred town tail.
            self.apply_pending_town_status_provision_pass();
            self.apply_pending_town_object_epilogue();
            self.append_pending_hourly_status_message();
        }
        Ok(MoveOutcome::Passed)
    }

    pub fn apply_top_down_post_turn_effects_after_turn(
        &mut self,
        turn_before: u64,
        game_dir: &Path,
    ) -> io::Result<Option<MoveOutcome>> {
        if self.turn == turn_before {
            return Ok(None);
        }
        let outcome = match self.area {
            Area::World { .. } => {
                self.apply_world_post_turn_effects_after_turn(turn_before, game_dir)
            }
            Area::Town { .. } => {
                self.apply_town_post_turn_effects_after_turn(turn_before, game_dir)
            }
            Area::Dungeon { .. } => {
                let outcome =
                    self.apply_dungeon_post_turn_effects_after_turn(turn_before, game_dir);
                // `stats-panel.md §2.2.1`: the dungeon's post-action redraw
                // is an immediate whole-panel site.
                self.repaint_stats_panel();
                outcome
            }
        }?;
        if outcome.is_none() {
            self.append_pending_hourly_status_message();
        }
        Ok(outcome)
    }

    pub fn apply_post_turn_effects_after_outcome(
        &mut self,
        turn_before: u64,
        game_dir: &Path,
        outcome: MoveOutcome,
    ) -> io::Result<Option<MoveOutcome>> {
        if outcome.is_transition() {
            Ok(None)
        } else {
            self.apply_top_down_post_turn_effects_after_turn(turn_before, game_dir)
        }
    }

    /// The post-action pass after a dungeon K-Klimb, or after an Uus Por /
    /// Des Por cast that changed the level.
    ///
    /// `dungeon-mode.md §13.1`: "A K that applies something - a climb up, a
    /// climb down, a pit fall ... - reports \"acted\" to the dungeon loop, so
    /// the turn's post-action pass runs." The level-change spells are the
    /// same transition - §13.1 "What a level change prints" gives them the
    /// climb's own word and nothing else - and a completed cast is an acted
    /// command too, so the same pass follows them (rc2 residual, cf.
    /// playtest E08). The spells accept only an open-passage destination
    /// ("The dungeon level-change spells"), so for them the underfoot arms
    /// find nothing, but the rest of the pass - the active monster's step,
    /// the immediate panel repaint, the hourly status line - ran a command
    /// late. That pass (`§4` step 6) reads the
    /// new underfoot cell, so a climb that lands on a fall trap falls and one
    /// that lands on a room cell enters it (`§8.1`, "Post-action underfoot
    /// consequences") before the next key is read. The generic
    /// [`Self::apply_post_turn_effects_after_outcome`] skips every
    /// transition, which deferred both to the next command (playtest E08).
    /// A climb that left the level stack is not in the dungeon any more and
    /// keeps that skip.
    pub fn apply_post_turn_effects_after_dungeon_level_change(
        &mut self,
        turn_before: u64,
        game_dir: &Path,
        outcome: MoveOutcome,
    ) -> io::Result<Option<MoveOutcome>> {
        let level_changed = matches!(
            outcome,
            MoveOutcome::Transition(AreaTransition::ChangedDungeonLevel { .. })
        );
        if !level_changed || !matches!(self.area, Area::Dungeon { .. }) {
            return self.apply_post_turn_effects_after_outcome(turn_before, game_dir, outcome);
        }
        // The climb word is already on the transcript (`climb_dungeon`), so
        // it is not a pending result the consequence lines must be joined to.
        // A pass that adds nothing leaves the word in the slot.
        let climb_word = (self.message == self.message_flushed).then(|| {
            (
                std::mem::take(&mut self.message),
                std::mem::take(&mut self.message_flushed),
            )
        });
        let mut outcome =
            self.apply_top_down_post_turn_effects_after_turn(turn_before, game_dir)?;
        if outcome.is_none() {
            outcome = self.resolve_current_dungeon_room_trigger(Some(game_dir))?;
        }
        if let Some((message, flushed)) = climb_word
            && self.message.is_empty()
        {
            self.message = message;
            self.message_flushed = flushed;
        }
        Ok(outcome)
    }

    pub fn apply_world_post_turn_effects_after_turn(
        &mut self,
        turn_before: u64,
        game_dir: &Path,
    ) -> io::Result<Option<MoveOutcome>> {
        if self.turn == turn_before {
            return Ok(None);
        }
        let Area::World { plane } = self.area else {
            return Ok(None);
        };

        let pre_effect_message = self.message.clone();
        let mut nonterminal_outcome = None;
        // `overworld.md` Section 8, precedence list item 3: the falls chain
        // is reached from the post-action pass when a waterfall is underfoot,
        // and from the top of the input helper when one stands directly
        // south. Both arms run the same handler, so this one site covers
        // both trigger cells (`RETRACTIONS.md` R320). It runs ahead of the
        // sidecar-driven plane transitions because the chain owns its own
        // plane write.
        // Underfoot only. `overworld.md §8` gives the chain two entry
        // points and splits them by trigger cell: "That is the arm a
        // waterfall directly south of the party takes; the post-action arm
        // is the one a waterfall underfoot takes." The directly-south arm
        // runs at the top of the input helper, in
        // [`PlayState::idle_wait_pass`], before the turn's key is read.
        if self.world_falls_underfoot_trigger_tile().is_some() {
            if let Some(transition) = self.apply_world_falls_chain(game_dir, plane)? {
                // A scene change ends the turn before the object epilogue;
                // the walker it owed must not run on the destination next
                // turn.
                self.pending_world_object_walker_pass = false;
                return Ok(Some(MoveOutcome::Transition(transition)));
            }
        }
        let pre_effect_row_open = self.message_row_open_mid_line;
        if let Some(transition) = self.apply_world_underfoot_plane_transition(game_dir, plane)? {
            self.pending_world_object_walker_pass = false;
            self.keep_post_turn_effect_rows_separate(&pre_effect_message, pre_effect_row_open);
            return Ok(Some(MoveOutcome::Transition(transition)));
        }
        let object_epilogue_runs = self.world_object_epilogue_runs_for_turn(turn_before);
        // `overworld.md §5` steps 6e-6g: the fixed narrative gate, the status
        // pass's report, then the exclusive underfoot chain (bridge ambush,
        // swamp, lava).
        self.apply_fixed_narrative_gate_branch(plane);
        self.append_pending_hourly_status_message();
        // `overworld.md §7.1`: "the first arm of the per-turn underfoot
        // chain", ahead of the swamp and lava arms below.
        self.apply_troll_bridge_ambush(plane);
        self.append_world_damage_tile_message(Some(game_dir), plane)?;
        self.append_world_status_tile_message(plane);
        // `overworld.md §6.2.5`: rough seas follows committed action work and
        // cleanup, but precedes the remaining encounter epilogue.
        let _ = self.apply_rough_seas_if_eligible();
        // `overworld.md §5` step 6h: "roll for a random encounter, then
        // animate/move and prune eligible non-player active objects".
        if object_epilogue_runs {
            if let Some(slot) = self.apply_world_encounter_probe(game_dir, plane)? {
                // The spawn is an engine-side event: `encounters.md §2.1`
                // rolls it every overworld turn and the original prints
                // nothing for it. Measured 2026-09-12
                // (`cast-blink-overworld/blinked`), where this engine
                // appended ` Wandering encounter spawned in slot 1.` to the
                // command's own result row.
                self.diagnostics
                    .push(format!("wandering encounter spawned in slot {slot}"));
            }
        }
        self.apply_pending_world_object_walker_pass();
        // active-objects.md §8: adjacent whirlpool engagement is a
        // plane-transition effect when the party is not on foot.
        let pre_effect_message = self.message.clone();
        let pre_effect_row_open = self.message_row_open_mid_line;
        if object_epilogue_runs {
            if let Some(outcome) = self.apply_pending_outdoor_reactions(game_dir, plane)? {
                if outcome.is_transition() {
                    self.keep_post_turn_effect_rows_separate(
                        &pre_effect_message,
                        pre_effect_row_open,
                    );
                } else if self.combat_active {
                    // `combat.md §4.1`: the engagement's `Attacked!`, group
                    // name and `*** CONFLICT ***` went to the transcript as
                    // they were printed, and each of those emits flushed the
                    // command's own result above them first. Joining the two
                    // back into the slot made it a new, unflushed line, so the
                    // next flush printed `With what? *** CONFLICT ***` a
                    // second time under the banner (playtest F42). The slot
                    // keeps the engagement text it already recorded.
                } else {
                    // Immediate reaction lines were emitted to the transcript
                    // as they occurred; preserve the command result in the
                    // compatibility message slot.
                    //
                    // A reaction line that replaced the slot was emitted
                    // through `emit_message_line`, which flushed the command's
                    // own result to the transcript first. The restored result
                    // is therefore already printed: mark it so, or the next
                    // flush prints it a second time - a pirate's `* BOOOM! *`
                    // was followed by an unmarked copy of `Fire-East`
                    // (playtest E28).
                    let reaction_replaced_slot = self.message != pre_effect_message;
                    self.message = pre_effect_message;
                    if reaction_replaced_slot {
                        self.message_flushed = self.message.clone();
                    }
                    nonterminal_outcome = Some(outcome);
                }
                if outcome.is_transition() || self.combat_active {
                    return Ok(Some(outcome));
                }
            }
        }
        Ok(nonterminal_outcome)
    }

    /// The row model for a post-turn effect that ends the turn in a plane
    /// transition (`overworld.md §8.1`): the command's own result and the
    /// effect's lines are separate rows, never one sentence. Joining them
    /// with a space (`"{command} {effect}"`) put a new, unflushed line in the
    /// slot that the next flush printed under rows already logged.
    ///
    /// - The effect printed nothing - the underfoot plane write "carries no
    ///   narration of its own" - so the slot keeps the command's row exactly
    ///   as it was, and the cursor stands where the command left it: the row
    ///   state is restored to `pre_effect_row_open`, the value the command
    ///   set.
    /// - The effect printed - the whirlpool's `\nWHIRLPOOL!\n` and the impact
    ///   payload's lines go through [`Self::emit_message_line`], which flushes
    ///   the command's row first, logs each line as its own row and sets the
    ///   row state from its literal. Anything the effect left in the slot
    ///   unlogged is flushed here as its own row
    ///   too. The row state is left exactly as the effect's own emitter set
    ///   it explicitly; it is never re-derived from the slot's text.
    /// - The effect only cleared the slot: that is not a print, so the
    ///   command's row comes back with the row state the command set.
    pub(crate) fn keep_post_turn_effect_rows_separate(
        &mut self,
        pre_effect_message: &str,
        pre_effect_row_open: bool,
    ) {
        if self.message == pre_effect_message || self.message.is_empty() {
            if self.message.is_empty() && !pre_effect_message.is_empty() {
                self.message = pre_effect_message.to_string();
            }
            self.message_row_open_mid_line = pre_effect_row_open;
            return;
        }
        if self.message != self.message_flushed {
            self.flush_message_slot();
        }
    }

    /// Resolve the high-to-low reaction list staged by the active-object
    /// walker. A terrain combat pauses this list in place; production input
    /// dispatch resumes it immediately after the combat frame returns.
    pub fn apply_pending_outdoor_reactions(
        &mut self,
        game_dir: &Path,
        plane: WorldPlane,
    ) -> io::Result<Option<MoveOutcome>> {
        let mut reacted = false;
        while !self.pending_outdoor_reaction_slots.is_empty() {
            let slot = self.pending_outdoor_reaction_slots.remove(0);
            let Some(object) = self.active_objects.get(slot).copied() else {
                continue;
            };
            if object.is_empty() || object.z != plane.save_floor() {
                continue;
            }

            if self.outdoor_active_object_is_adjacent(slot) {
                if is_whirlpool_object(object) {
                    let outcome =
                        self.apply_world_whirlpool_slot_engagement(game_dir, plane, slot)?;
                    reacted = true;
                    if outcome.is_transition() {
                        self.pending_outdoor_reaction_slots.clear();
                        return Ok(Some(outcome));
                    }
                    continue;
                }
                if outdoor_sand_trap_class(object.type_byte) {
                    self.apply_world_sand_trap_slot_engagement(slot);
                    reacted = true;
                    continue;
                }
                if let Some(outcome) = self
                    .apply_world_generic_adjacent_slot_engagement(game_dir, plane, slot, object)?
                {
                    reacted = true;
                    if self.combat_active {
                        return Ok(Some(outcome));
                    }
                }
                continue;
            }

            if self
                .outdoor_claimed_ranged_attack_detail(slot)
                .is_some()
            {
                reacted = true;
            }
        }
        Ok(reacted.then_some(MoveOutcome::Used))
    }

    pub fn apply_world_whirlpool_engagement(
        &mut self,
        game_dir: &Path,
        plane: WorldPlane,
    ) -> io::Result<Option<MoveOutcome>> {
        let Some(whirlpool_slot) = (1..self.active_objects.len()).rev().find(|slot| {
            self.outdoor_active_object_is_adjacent(*slot)
                && is_whirlpool_object(self.active_objects[*slot])
        }) else {
            return Ok(None);
        };
        self.apply_world_whirlpool_slot_engagement(game_dir, plane, whirlpool_slot)
            .map(Some)
    }

    fn apply_world_whirlpool_slot_engagement(
        &mut self,
        game_dir: &Path,
        plane: WorldPlane,
        whirlpool_slot: usize,
    ) -> io::Result<MoveOutcome> {
        // `overworld.md §8` correction: both marker arms reach the shared
        // §6.2.4 impact payload. On foot this is the whole-party damage pass;
        // it skips only the transition and does not clear the whirlpool slot.
        // `overworld.md §6.2.4` (u5-spec#301 Q3, R612): both whirlpool arms
        // enter the payload at stage two, with no impact figure or rumble.
        if self.player.transport.is_foot() {
            self.apply_outdoor_impact_absorption();
            return Ok(MoveOutcome::Used);
        }

        // Apply impact before changing planes so absorption sees the restored
        // original transport marker. A frigate therefore takes its hull roll
        // (and can sink) immediately before the durable transition.
        // `overworld.md` Section 8.1, whirlpool swallow, in print order.
        // Step 1: the banner, with its leading line feed - one blank row.
        // "There is **no advance warning line**: the swallow banner ... is the
        // first and only text."
        self.emit_overworld_whirlpool_banner();
        // Step 2: the slot is cleared and the party marker is replaced by the
        // whirlpool sprite.
        let whirlpool_tile = self.active_objects[whirlpool_slot].tile;
        self.free_active_object_slot(whirlpool_slot);
        self.party_marker_tile_override = Some(whirlpool_tile);
        self.sync_player_object();
        self.mark_visibility_dirty();
        // Step 3: "One world tick, so the player sees a whirlpool where the
        // party was."
        self.advance_visual_tick();
        // Step 4: the long descending sweep.
        // `audio.md §8.9`, second row: "The whirlpool object is cleared,
        // `WHIRLPOOL!` prints, the party sprite is **replaced by the whirlpool
        // sprite**, and the viewport repaints ... **then** the long descent -
        // then the sprite is restored, the shared impact payload runs, and the
        // party is teleported".
        //
        // So the sweep sits after the slot is freed and strictly before the
        // impact payload and the transition: "The state commit - the teleport -
        // happens strictly after it." The on-foot arm above returned already,
        // which is the `§9` silence boundary "whirlpool engagement while the
        // party is on foot, which plays no long descent".
        self.emit_sound_effect(SoundEffect::LongDescent);
        // Step 5: "The party marker is restored", before the impact payload,
        // which is why a frigate's hull roll can still sink the ship in the
        // instant before the teleport.
        self.party_marker_tile_override = None;
        self.sync_player_object();
        self.mark_visibility_dirty();
        // Step 6: the Section 6.2.4 impact payload, "which may add
        // `Ship sunk!` and then either `Abandon ship!` or `DROWNING!!!`".
        // It enters at the absorption stage (R612, u5-spec#301 Q3).
        self.apply_outdoor_impact_absorption();
        // Step 7: "The plane write and the move to `(34, 18)` ... This runs
        // **unconditionally** after step 6, including after the drowning
        // arm." It prints nothing of its own: the banner above is the first
        // and only text on the path, so the coordinate narration this used to
        // emit is removed.
        let entry = WorldPlaneTransitionEntry {
            from_plane: plane,
            x: self.player.x,
            y: self.player.y,
            to_plane: WorldPlane::Underworld,
            to_x: WHIRLPOOL_UNDERWORLD_EMERGENCE_X,
            to_y: WHIRLPOOL_UNDERWORLD_EMERGENCE_Y,
            expected_tile: None,
            // `vehicles.md §2`: the whirlpool frame is a one-frame override
            // that is "saved ... and restored", and the marker writer census
            // has no whirlpool reset to foot. Section 8.1 restores the marker
            // before the payload, so only the payload's own sinking ladder
            // may change it; the teleport keeps what it left.
            preserves_transport: true,
        };
        self.apply_world_plane_transition(game_dir, entry)?;
        Ok(MoveOutcome::Transition(AreaTransition::ChangedWorldPlane {
            from: plane,
            to: WorldPlane::Underworld,
        }))
    }

    pub fn apply_world_sand_trap_engagement(&mut self, plane: WorldPlane) -> Option<MoveOutcome> {
        let sand_trap_slot = (1..self.active_objects.len()).rev().find(|slot| {
            let object = self.active_objects[*slot];
            if object.z != plane.save_floor() || !outdoor_sand_trap_class(object.type_byte) {
                return false;
            }
            let (dx, dy) = wrapped_deltas_to_player(
                object.x as u8,
                object.y as u8,
                self.player.x as u8,
                self.player.y as u8,
            );
            outdoor_offsets_are_orthogonally_adjacent(dx, dy)
        })?;

        // `active-objects.md §8`: the sand-trap adjacency arm reaches the
        // shared impact payload directly and silently. It is not combat and
        // does not clear the active-object slot.
        self.apply_world_sand_trap_slot_engagement(sand_trap_slot);
        Some(MoveOutcome::Used)
    }

    fn apply_world_sand_trap_slot_engagement(&mut self, sand_trap_slot: usize) {
        let _ = sand_trap_slot;
        // `active-objects.md §8` (u5-spec#301 Q3, R612): "no impact figure
        // or impact rumble" - the payload's absorption stage only.
        self.apply_outdoor_impact_absorption();
    }

    pub fn apply_town_post_turn_effects_after_turn(
        &mut self,
        turn_before: u64,
        game_dir: &Path,
    ) -> io::Result<Option<MoveOutcome>> {
        if self.turn == turn_before {
            return Ok(None);
        }
        // `active-objects.md §9`: "Combat suspends the world by swapping
        // the active-object table to a backup region and overwriting the
        // live table with combat actors ... The calling mode loop sees
        // combat as a function call that returns with the table and
        // globals exactly as they were". The §7 per-turn epilogue -
        // underfoot effects and the NPC schedule processor - therefore
        // resumes only once the framer has restored the world table. This
        // arm became reachable with the §14 NPC-conflict chain, which
        // enters an arena while the area is still a town.
        if self.combat_active {
            return Ok(None);
        }
        let Area::Town { scene, floor } = self.area else {
            return Ok(None);
        };
        // `town-mode.md §7`: the town underfoot handler starts by giving
        // every sleeping active member an independent 1-in-16 wake roll.
        // This intentionally precedes trapdoor damage and every other tile
        // effect, including on the automatic no-input sleep pass.
        let mut waking = Vec::new();
        for (index, member) in self.party.iter().enumerate() {
            if member.status == b'S'
                && u5_prng_range_u16(&mut self.prng_state, 0, u16::from(TOWN_SLEEP_WAKE_ROLL_MAX))
                    == 0
            {
                waking.push(index);
            }
        }
        // No panel repaint here, and no request either. **Measured**
        // 2026-10-19, pending `cleak/u5-spec#299` Q23
        // (`qa/paired/asleep-town.tsv`): the stock town party wakes within
        // a few passes - Space is then accepted as `Pass` - yet the panel
        // keeps showing all three members `S` through three minutes and
        // twelve Passes. The combat wake does repaint (`asleep-overworld`).
        //
        // The wake is per-turn work, not the command's: `stats-panel.md
        // §2.2` "Nothing refreshes the panel at a turn boundary" and §2.3
        // "Town consumes but never raises". The dispatcher's end-of-command
        // comparison would otherwise attribute the status change to the
        // command (`Pass` is an immediate site by default) and repaint `G`.
        if !waking.is_empty() {
            self.wake_town_sleepers_without_panel_refresh(&waking);
        }
        let tile = self.grid[self.player.y * 32 + self.player.x];
        if !matches!(self.player.transport, TransportState::Carpet { .. }) {
            if let Some(entry) =
                self.town_trap_door_at(game_dir, scene, floor, self.player.x, self.player.y, tile)?
            {
                let step_sound_serial = self.sound_effect_serial;
                self.apply_town_trapdoor_party_damage();
                if scene.byte == STONEGATE_SCENE_BYTE {
                    // `town-mode.md §7.1` orders the script: the black fill
                    // and the 750-tone speaker sweep come first, and only
                    // after them does the death loop run - `audio.md §8.2`
                    // puts "one 75-update 100..500 Hz rumble per party member,
                    // as that member is killed and the stats panel is
                    // repainted" *inside* that loop.
                    //
                    // So the panel the player looks at during the sweep still
                    // carries the fall's damage and not the deaths. Measured
                    // 2026-09-18 (`qa/paired/stonegate-rescue-pacing.tsv`,
                    // samples `t04` through `t24`): the original's roster row
                    // reads `Avatar 54G` for the whole sweep - damaged from
                    // 60, not dead - and only reads `0D` from `t28`, once the
                    // sweep is over. This engine applied the deaths and
                    // repainted at the end of the step, so it read `0D`
                    // throughout.
                    //
                    // Paint the damage here, where the sweep sees it, and
                    // decline the command's own end-of-step refresh so the
                    // deaths stay off until the rescue takes over. That is
                    // what the dispatcher's note asks a named hold to do.
                    self.repaint_stats_panel();
                    self.stats_panel_command_refresh =
                        crate::stats_panel::PanelRefreshMode::None;
                    self.apply_stonegate_trapdoor_script_after(floor, step_sound_serial);
                    self.emit_town_underfoot_line(Self::TOWN_TRAPDOOR_MESSAGE);
                    // The line closes its own row. `text-output.md` §10.4
                    // then has the rescue's first beat spend its leading feed
                    // deriving the blank beneath rather than closing a row
                    // this one already closed.
                    //
                    // Measured 2026-09-18
                    // (`qa/paired/stonegate-rescue-pacing.tsv`): the original
                    // holds a blank row under `A TRAPDOOR!` across six
                    // consecutive samples of the death sweep, and at `t28`
                    // keeps it between `A TRAPDOOR!` and `An unending
                    // darkness engulfs thee...`. This engine ran the two
                    // together and sat one row high from there on.
                    // `cleak/u5-spec#279` asks for the published framing;
                    // this is the capture.
                    return Ok(Some(MoveOutcome::Used));
                }
                let mut outcome =
                    self.apply_town_trap_door_transition(game_dir, scene, entry, false)?;
                // `town-mode.md §10`: "If an effect moves the party to a
                // different floor, the handler re-reads the tile under the
                // party's new position and applies that tile's effect too, so
                // chained effects within one turn are possible" - including a
                // landing on another trapdoor, which runs the whole trapdoor
                // prefix and descends again. The chain stops once a landing
                // no longer changes the floor.
                loop {
                    let landed_tile = self.grid[self.player.y * 32 + self.player.x];
                    let Area::Town {
                        floor: landed_floor,
                        ..
                    } = self.area
                    else {
                        break;
                    };
                    if !matches!(self.player.transport, TransportState::Carpet { .. })
                        && let Some(next) = self.town_trap_door_at(
                            game_dir,
                            scene,
                            landed_floor,
                            self.player.x,
                            self.player.y,
                            landed_tile,
                        )?
                        && next.to_floor != landed_floor
                    {
                        self.apply_town_trapdoor_party_damage();
                        outcome =
                            self.apply_town_trap_door_transition(game_dir, scene, next, false)?;
                        continue;
                    }
                    if Self::is_town_burning_live_tile(landed_tile) {
                        self.apply_town_burning_underfoot_effect();
                    }
                    break;
                }
                let Area::Town {
                    scene: landed_scene,
                    floor: landed_floor,
                } = self.area
                else {
                    unreachable!("town trapdoor transition must remain in town mode");
                };
                self.append_town_poison_gas_message(game_dir, landed_scene, landed_floor)?;
                self.apply_pending_town_status_provision_pass();
                self.apply_pending_town_object_epilogue();
                return Ok(Some(outcome));
            }
        }
        // `town-mode.md §10` burning family. This arm runs after the
        // trapdoor arm and before the trailing party pass, and it is not
        // shadowed by the trapdoor: the trapdoor tile is `0x8C` while the
        // burning ids are `0xBC`/`0x8F`. It has no transport gate, so it
        // fires from a carpet too, and it re-fires every consumed turn the
        // party spends standing on the tile.
        if Self::is_town_burning_live_tile(tile) {
            self.apply_town_burning_underfoot_effect();
        }
        // `town-mode.md §17` "Underfoot-effect cadence is fixed": "The
        // underfoot handler is a per-turn post-action pass, not a step-commit
        // hook. Any earlier statement that the poison-gas effect 'fires from
        // the step path' is retracted: it fires once per turn-consuming action
        // while the party occupies the tile, including turns spent passing in
        // place, and it fires after that turn's clock advance."
        //
        // `town-mode.md §10`: "If an effect moves the party to a different
        // floor, the handler re-reads the tile under the party's new position
        // and applies that tile's effect too", so the gas arm re-reads the
        // live tile rather than reusing the `tile` sampled above the trapdoor
        // arm.
        self.append_town_poison_gas_message(game_dir, scene, floor)?;
        self.apply_pending_town_status_provision_pass();
        self.apply_pending_town_object_epilogue();
        self.apply_town_npc_contact_event(scene, floor, game_dir)
    }

    /// Finish the shared pass deliberately deferred by an ordinary town
    /// turn's clock advance. `town-mode.md §10` makes this the last act of the
    /// underfoot handler, after waking, trapdoor/burning, and poison-gas work.
    pub(crate) fn apply_pending_town_status_provision_pass(&mut self) {
        if std::mem::take(&mut self.pending_town_status_provision_pass) {
            self.apply_hourly_status_provision_pass();
        }
    }

    /// Finish the scheduler/object half of the ordinary town epilogue after
    /// underfoot effects and the shared status/provision pass.
    ///
    /// `town-mode.md §7` step 4, as corrected by **R328**: the turn's
    /// movement work runs "in a fixed order: three effect gates, the **town
    /// object walker** that moves the location's loose horse-family objects,
    /// and finally the NPC schedule processor with the current hour byte".
    /// The three gates were applied by the clock routine, which is what left
    /// these two flags set; this is the walker half of that order.
    ///
    /// The explicit-T arrest discriminator is the fourth skip, and it is the
    /// only one of the four that, in `town-mode.md §7` step 4's words,
    /// "skips the schedule processor *only*, and it is tested after the
    /// object walker has already made its pass". `npc-schedules.md §5` draws
    /// the consequence: "That is why a result-two turn can still move a loose
    /// horse-family object while no scheduled NPC moves." Both walkers
    /// raise the visibility-dirty flag from inside themselves, so §7 step 5's
    /// "the test has to cover **both** walkers, not just the schedule
    /// processor" holds on a result-two turn too.
    ///
    /// *Corrected (R328).* This used to call the schedule processor first and
    /// the object pass afterwards, which left a repaint conditioned on the
    /// processor's report blind to an object the walker had moved.
    pub(crate) fn apply_pending_town_object_epilogue(&mut self) {
        // The tail belongs to the town mode loop, which is suspended while
        // an arena fight runs: during combat the active-object table holds
        // the arena's actors (`combat.md §4`), and the walkers would move
        // them and the schedule processor would write town NPCs into it.
        // Anything still owed when an arena opens stays owed until the
        // frame is restored ([`Self::drain_town_tail_after_combat`]).
        if self.combat_active {
            return;
        }
        let run_npc_schedule = std::mem::take(&mut self.pending_town_npc_schedule_pass);
        let run_active_objects = std::mem::take(&mut self.pending_town_active_object_pass);
        // `npc-schedules.md §5` gates "**both** town walkers", which the
        // animator is not one of (`npc-schedules.md §12`; `RETRACTIONS.md`
        // R316), so the animate half carries its own flag and is not skipped
        // by a turn the transport or Quickness parity gated out.
        let run_animator = std::mem::take(&mut self.pending_town_active_object_animate_pass);
        if run_animator {
            self.animate_active_objects();
        }
        if run_active_objects {
            self.advance_active_object_walkers();
        }
        if run_npc_schedule && self.pending_town_arrest.is_none() {
            self.advance_npc_schedules();
        }
    }

    /// Run any town walker tail a fight left owed, once the arena frame has
    /// been restored.
    ///
    /// [`Self::apply_pending_town_object_epilogue`] refuses to run against
    /// the arena's actor table, so a pass still pending when combat began
    /// waits for this. A town A-Attack no longer leaves one: its turn is
    /// spent after the fight ([`Self::run_town_attack_epilogue_after_combat`]).
    /// `town-mode.md §14`'s conflict chain clears the NPC slot and reloads
    /// the map before the handler returns, so this drains after
    /// [`Self::drain_pending_town_conflict`].
    pub fn drain_town_tail_after_combat(&mut self) {
        if self.combat_active || !matches!(self.area, Area::Town { .. }) {
            return;
        }
        self.apply_pending_town_object_epilogue();
    }

    /// `town-mode.md §7` step 4, "An A-Attack that fights in the arena"
    /// (issue #298 Q40): the whole fight, the slot clear, the town-map reload
    /// and the attach rerun finish inside the command, and only then does the
    /// turn's epilogue run - **once**: "one indoor minute, dawn/dusk if due,
    /// the underfoot and status pass, the auto-close countdown, the slot-zero
    /// copy, the three gates, the object walker, the schedule pass, and
    /// post-action cleanup if a contact was raised". Victory and flight look
    /// the same here.
    ///
    /// Ahead of it the town loop re-runs the party-capability check: "If
    /// nobody can act and nobody is asleep, the **whole** epilogue is skipped
    /// ... The next iteration's opening check then enters the total-party-
    /// defeat rescue", and "an all-asleep result skips nothing here".
    ///
    /// The auto-close step is a no-op after the fight on its own: the round
    /// loop and the post-fight reload both clear the tracker's gating byte
    /// (`doors-and-z-transitions.md §5`).
    pub fn run_town_attack_epilogue_after_combat(
        &mut self,
        game_dir: &Path,
    ) -> io::Result<Option<MoveOutcome>> {
        if self.combat_active || !matches!(self.area, Area::Town { .. }) {
            return Ok(None);
        }
        if matches!(self.party_capability(), PartyCapability::Defeated) {
            return Ok(None);
        }
        let turn_before = self.turn;
        self.advance_turn();
        self.apply_top_down_post_turn_effects_after_turn(turn_before, game_dir)
    }

    /// `town-mode.md §10`: the live town tiles in the burning family — the
    /// fireplace `0xBC` and molten lava `0x8F` of
    /// `catalogs/tile-catalog.md §6`. The earlier "rune/lever family" label
    /// for this bullet is withdrawn; both ids are damage tiles.
    pub const fn is_town_burning_live_tile(tile: u8) -> bool {
        matches!(
            tile,
            Self::TOWN_BURNING_FIREPLACE_TILE | Self::TOWN_BURNING_LAVA_TILE
        )
    }

    /// `town-mode.md §10`: "Rebuild the view, print the stored line
    /// `Burning!`, then apply the same independently rolled `1..8` mass
    /// damage to every non-Dead slot."
    pub fn apply_town_burning_underfoot_effect(&mut self) {
        self.mark_visibility_dirty();
        self.emit_town_underfoot_line(Self::TOWN_BURNING_MESSAGE);
        self.apply_town_burning_party_damage();
    }

    /// `town-mode.md §10` (issue #279): `A TRAPDOOR!`, `Burning!` and
    /// `Poisoned!` are each "one terminated string ... with a single trailing
    /// line feed and no leading one", printed as stored. Each therefore lands
    /// on its own row and leaves the cursor at column 0 of the next one.
    ///
    /// They used to be space-joined into the message slot - onto the
    /// command's own result and onto one another - which put several
    /// published lines on one row. The command's verb echo, when one is
    /// still pending, is committed first so the command's result stays on
    /// its verb row above this line rather than being flushed onto a row of
    /// its own. The row state is set explicitly rather than inferred from
    /// the text.
    pub(crate) fn emit_town_underfoot_line(&mut self, line: &str) {
        self.commit_command_echo();
        self.emit_message_line(line);
        self.close_message_row();
    }

    /// The burning family's mass damage is the same independently rolled
    /// `1..8` pass the trapdoor arm runs (`town-mode.md §10`).
    pub fn apply_town_burning_party_damage(&mut self) {
        let slots = self.party.len().min(COMBAT_PARTY_ACTOR_SLOTS);
        for slot in 0..slots {
            if self.party[slot].status == CharacterStatus::Dead.save_byte() {
                continue;
            }
            let damage = self.random_range_u8(1, TRAP_BOMB_DAMAGE_MAX);
            self.apply_shared_party_damage(slot, damage);
        }
    }

    pub fn apply_town_trapdoor_party_damage(&mut self) {
        let slots = self.party.len().min(COMBAT_PARTY_ACTOR_SLOTS);
        for slot in 0..slots {
            if self.party[slot].status == CharacterStatus::Dead.save_byte() {
                continue;
            }
            let damage = self.random_range_u8(1, TRAP_BOMB_DAMAGE_MAX);
            self.apply_shared_party_damage(slot, damage);
        }
    }

    /// Stonegate's trapdoor exception after the shared mass-damage pass.
    ///
    /// Public issue `cleak/u5-spec#123`, resolved at clean-spec commit
    /// `4d03a662`, confirms that this is a same-scene scripted defeat rather
    /// than a floor transition or a durable rescue flag. The presentation is
    /// recorded separately; the durable mutations here match the cutscene's
    /// live-grid, active-object, and party writes.
    pub fn apply_stonegate_trapdoor_script(&mut self, floor: i8) {
        self.apply_stonegate_trapdoor_script_after(floor, self.sound_effect_serial);
    }

    /// [`Self::apply_stonegate_trapdoor_script`], with the step's sounds
    /// counted from `step_sound_serial`: the fall's damage rumbles play ahead
    /// of the descent, and the hold has to outlast them too.
    pub(crate) fn apply_stonegate_trapdoor_script_after(
        &mut self,
        floor: i8,
        step_sound_serial: u64,
    ) {
        self.pending_stonegate_trapdoor_playback =
            Some(StonegateTrapdoorPlayback::complete(self.party.len()));

        // `town-mode.md §7.1` fixes the order of this script precisely, and
        // `audio.md §8.2` places the descent inside it. Step 1 is the direct
        // black viewport fill, carried by the playback record above. Step 2 is
        // the speaker sweep — every integer frequency from 1000 down through
        // 251 Hz, 750 tones. Only then does step 3 rewrite the live grid.
        // Emitting after the grid rewrite would put the sweep one published
        // step late.
        self.emit_sound_effect(SoundEffect::StonegateDescent);

        self.grid.fill(STONEGATE_TRAPDOOR_GRID_TILE);
        self.mark_visibility_dirty();
        // Step 1's black viewport fill is a direct-screen operation and holds
        // until the rescue hands off - the live grid underneath it is the
        // `0x8f` fill above, which the player never sees. Measured 2026-09-18
        // (`qa/paired/stonegate-rescue-pacing.tsv`, every sample from `t04`):
        // the original's viewport is entirely black through the sweep, and
        // this engine drew the `0x8f` fill with the party standing on it,
        // because the black fill was a one-shot playback the frontend drained
        // after a single frame.
        self.cutscene_black_viewport = true;

        self.active_objects = vec![ActiveObject::empty(); OOL_SLOTS];

        for slot in 0..self.party.len() {
            self.party[slot].hp = 0;
            self.party[slot].status = CharacterStatus::Dead.save_byte();
            // `audio.md §8.2`: the descent stops, then one 75-update
            // 100..500 Hz rumble per party member, as that member is killed
            // and the stats panel is repainted.
            self.emit_sound_effect(SoundEffect::StonegateMemberDeath);
        }

        // The sweep is blocking, so nothing after the death runs until it has
        // finished. Staged as a pure hold - no text - which the rescue's own
        // beats then queue behind. Pushed once every sound of the step has
        // been queued; nothing else stages between here and the rescue.
        // [`crate::stonegate_trapdoor_death_hold_bios_ticks`] carries the
        // duration and why.
        let step_sounds = self.sound_effects_after(step_sound_serial);
        self.staged_narration.push_ticks(
            crate::stonegate_trapdoor_death_hold_bios_ticks(&step_sounds),
            String::new(),
        );

        // The status/provision tail follows the deaths. If this action crossed
        // an hour boundary, its normal pass was deferred by `advance_turn` so
        // it observes zero provision eaters and no regenerating/poisoned live
        // members. The same tail clears an in-party active-member selector.
        if self
            .active_player
            .is_some_and(|slot| slot < self.party.len())
        {
            self.active_player = None;
        }
        self.apply_pending_town_status_provision_pass();

        // The normal coordinate-only record-zero tail follows the all-zero
        // script boundary. The zero-type animal pass cannot move it; the NPC
        // schedule then retains its ordinary opportunity to write records.
        self.active_objects[0].x = self.player.x;
        self.active_objects[0].y = self.player.y;
        self.active_objects[0].z = floor;
        self.apply_pending_town_object_epilogue();
    }

    pub fn town_poison_gas_at(
        &self,
        _game_dir: &Path,
        scene: Scene,
        floor: i8,
        x: usize,
        y: usize,
        tile: u8,
    ) -> io::Result<Option<TownPoisonGasEntry>> {
        let transport_marker = self.player.transport.save_marker();
        if town_poison_gas_live_tile_matches(tile, transport_marker) {
            return Ok(Some(TownPoisonGasEntry {
                scene,
                floor,
                x,
                y,
                expected_tile: Some(tile),
            }));
        }
        Ok(None)
    }

    /// `town-mode.md §10` poison-gas terrain, driven at the `§17` cadence:
    /// "The underfoot handler is a per-turn post-action pass, not a
    /// step-commit hook ... it fires once per turn-consuming action while the
    /// party occupies the tile, including turns spent passing in place, and it
    /// fires after that turn's clock advance." The only caller is
    /// [`PlayState::apply_town_post_turn_effects_after_turn`]; the retracted
    /// step-commit call site in the town step path is gone.
    pub fn append_town_poison_gas_message(
        &mut self,
        game_dir: &Path,
        scene: Scene,
        floor: i8,
    ) -> io::Result<()> {
        let tile = self.grid[self.player.y * 32 + self.player.x];
        let Some(entry) =
            self.town_poison_gas_at(game_dir, scene, floor, self.player.x, self.player.y, tile)?
        else {
            return Ok(());
        };
        self.apply_town_poison_gas(entry);
        Ok(())
    }

    /// `town-mode.md §10` poison-gas save. For each eligible slot in order,
    /// a roll above Dexterity sets Poisoned, prints the status line and
    /// repaints the stats panel. "Each failed save prints exactly
    /// `Poisoned!\n`, once for that member, with no name, number or
    /// gas-specific lead-in ... Skipped members and successful saves add no
    /// text." Returns how many members were poisoned.
    pub fn apply_town_poison_gas(&mut self, _entry: TownPoisonGasEntry) -> usize {
        let mut poisoned = 0usize;
        for index in 0..self.party.len() {
            let status = self.party[index].status;
            if status == b'D' || status == b'P' {
                continue;
            }
            let roll = u5_prng_range_u16(&mut self.prng_state, 0, TOWN_GAS_DOORWAY_RANGE_MAX);
            if roll <= u16::from(self.party[index].climb_stat) {
                continue;
            }
            self.party[index].status = b'P';
            poisoned += 1;
            // One terminated `Poisoned!\n` row per failing member.
            self.emit_town_underfoot_line(Self::TOWN_POISONED_MESSAGE);
            self.repaint_stats_panel();
        }
        poisoned
    }

    /// The cardinal direction from the party to an orthogonally adjacent
    /// cell, for the conversation-contact dispatch. Returns `None` when the
    /// cell is not orthogonally adjacent, which the producer's own adjacency
    /// test already excludes.
    fn contact_direction_to(&self, x: usize, y: usize) -> Option<Direction> {
        crate::npc_runtime::town_direction_from_party(self.player.x, self.player.y, x, y)
    }

    /// `npc-schedules.md §9.2`'s contact-event routing table.
    ///
    /// The engine used to run the withdrawn reading (`RETRACTIONS.md` R399):
    /// AI `4`/`5`/`7` raised an *attack* event that swept the town alarm, and
    /// only AI `6` was a guard. Since every shipped shopkeeper carries AI `4`
    /// at its working waypoint, walking up to one raised the alarm and left
    /// half the roster cowering - `cleak/u5-engine#16`, and the same defect
    /// behind `#15`. The table below is §9.2's.
    pub fn apply_town_npc_contact_event(
        &mut self,
        scene: Scene,
        floor: i8,
        game_dir: &Path,
    ) -> io::Result<Option<MoveOutcome>> {
        // `npc-schedules.md §9.2`: the event is the one the schedule pass
        // recorded at the start of its producer's engagement invocation -
        // "the last producer visited supplies the event that town mode
        // handles". A turn whose pass did not run has none to handle.
        let Some(event) = self.pending_town_npc_contact.take() else {
            return Ok(None);
        };
        if floor < 0 {
            return Ok(None);
        }
        // `npc-schedules.md §9.2` / `commands.md §3`: contact is handled in
        // the epilogue *after* the command, so it adds to that command's
        // output and never replaces it. Push, Open, Jimmy and Attack leave
        // their result (`Won't budge!`, `Nothing to open!`, ...) in the
        // message slot for the turn boundary's flush, and the conversation
        // and brush-off arms below overwrite that slot - the result line
        // was lost whenever a contact fired on the same turn (playtest
        // K16). Commit it to the transcript first.
        self.flush_message_slot();
        let npc_slot = event.npc_slot;
        let behavior = event.behavior;
        let Some(type_byte) = self
            .npcs
            .iter()
            .find_map(|npc| (npc.slot == npc_slot).then_some(npc.type_byte))
        else {
            return Ok(None);
        };
        // §9.2: the brush-off belongs to the **arrest/conflict** family -
        // "AI 6 or 7 | Dialogue `0xFE` | Shouted brush-off followed by that
        // NPC's flight rewrite."
        if behavior.raises_arrest_or_conflict_contact()
            && let Some(index) = self.npcs.iter().position(|npc| npc.slot == npc_slot)
            && self.npcs[index].dialog_id == TOWN_NPC_BRUSHOFF_DIALOG_ID
        {
            // `town-mode.md §14` / `npc-schedules.md` producer table
            // (u5-spec#300 Q19): "one line feed, the line, the same damage
            // pass, then flight for that NPC. There is no `Attacked!`, no
            // conflict and no arrest."
            self.message_row_open_mid_line = false;
            self.emit_message_line(format!("\n{TOWN_NPC_BRUSHOFF_LINE}\n"));
            let _ = self.apply_outdoor_impact_party_damage();
            let _ = self.npcs[index].force_town_flight();
            self.record_town_npc_mutation(index);
            self.push_diagnostic(format!("NPC slot {npc_slot} shouts the brush-off"));
            return Ok(Some(MoveOutcome::Used));
        }
        // Measured 2026-09-08 at Minoc's gate at 16:00
        // (`qa/paired/minoc-guard.tsv`): the regime demand fires when the
        // party walks up to the guard, and that guard's waypoint behaviour is
        // the **approach-and-attack** value 4, not the guard/block value 6.
        // The demand dialog byte is what marks a regime guard, so it is
        // checked before the behaviour split rather than inside the
        // guard-event arm; the engine used to reach the demand only through
        // Talk, and walking into Minoc drew nothing at all.
        if behavior.raises_conversation_contact()
            && let Some((x, y)) = self.npcs.iter().find_map(|npc| {
                (npc.slot == npc_slot && npc.dialog_id == BLACKTHORN_GUARD_DEMAND_DIALOG_ID)
                    .then_some((npc.x, npc.y))
            })
        {
            // `conversation.md §2`: contact enters the same dispatcher, whose
            // behaviour-4-to-1 rewrite runs before the regime handler.
            self.talk_dispatcher_stand_down(x, y);
            return Ok(Some(self.begin_blackthorn_guard_demand(
                x,
                y,
                false,
                Some(game_dir),
            )?));
        }
        // §9.2, first row: "AI 4 or 5 | Nonzero live dialogue | Enter the
        // shared conversation dispatcher for that NPC." The zero-dialogue
        // case never reaches here - the schedule pass records no event for
        // it (`town_npc_contact_dialogue_allows`), as
        // §9.2's second row requires ("No contact event; continue movement
        // selection. This is not a fallback to combat").
        //
        // "Conversation contact uses the **same dispatcher as explicit Talk**,
        // but does not perform Talk's direction, target lookup or mirror/bed
        // terrain checks." This engine's dispatcher is direction-keyed, and
        // the producer is orthogonally adjacent by construction, so the
        // direction is derived rather than prompted.
        if behavior.raises_conversation_contact() {
            let Some((npc_x, npc_y)) = self
                .npcs
                .iter()
                .find_map(|npc| (npc.slot == npc_slot).then_some((npc.x, npc.y)))
            else {
                return Ok(None);
            };
            let Some(direction) = self.contact_direction_to(npc_x, npc_y) else {
                return Ok(None);
            };
            // `shops.md §2`: automatic entry "does not create an additional
            // Talk command or schedule tick".
            self.talk_entered_by_contact = true;
            let outcome = self.talk_direction_with_game_dir(direction, game_dir);
            self.talk_entered_by_contact = false;
            let outcome = outcome?;
            self.push_diagnostic(format!(
                "NPC slot {npc_slot} (type {type_byte}) opens conversation contact."
            ));
            return Ok(Some(outcome));
        }
        if !behavior.raises_arrest_or_conflict_contact() {
            return Ok(None);
        }
        // `npc-schedules.md §9.2` / `town-mode.md §14`: past the brush-off,
        // "the exact linked live guard sprite byte `0x70` enters arrest;
        // other sprites take the conflict outcome. At that outcome, an actor
        // byte at least `0x40` prints `Attacked!` and enters the NPC-conflict
        // chain; a lower byte clears the NPC slot." The live sprite is byte 0
        // of the NPC's linked active-object record (`active-objects.md §3`).
        let linked = self.npcs.iter().find_map(|npc| {
            (npc.slot == npc_slot)
                .then_some(npc.active_object)
                .flatten()
                .and_then(|slot| {
                    self.active_objects
                        .get(slot)
                        .copied()
                        .filter(|object| !object.is_empty())
                        .map(|object| (slot, object))
                })
        });
        let live_sprite = linked.map_or(type_byte, |(_, object)| object.type_byte);
        if live_sprite == crate::tlk_control_codes::TALK_GUARD_SPRITE {
            // `town-mode.md §14` / `blackthorn.md §3`: "The arrest sequence
            // itself branches on the current location **before it prints
            // anything**. Inside Lord Blackthorn's Castle, and while the
            // shared party-capability check reports that at least one member
            // can act or is asleep, it plays the Blackthorn audience/capture
            // cinematic ... In every other location it prints the arrest
            // challenge."
            //
            // This engine opened the challenge everywhere and reached the
            // audience only if the player then answered `Y`, so walking up
            // the palace approach read `"Thou art under arrest!"` /
            // `"Wilt thou come quietly?"` where the stock game was already
            // asking the first mantra. Caught at `bt-audience`'s `ask1` beat
            // by `qa/tools/paired_compare.py` (`cleak/u5-engine#21`).
            let arrest = TownArrestPrompt {
                scene_byte: scene.byte,
                floor,
                npc_slot,
            };
            if scene.byte == crate::blackthorn::BLACKTHORN_CAPTIVE_CELL_SCENE
                && self.party_capability() == PartyCapability::Defeated
            {
                // `RETRACTIONS.md` R442: with nobody able to act and nobody
                // asleep the helper "returns silently without an audience or
                // surrender prompt"; the exploration loop's own defeat check
                // owns the rescue from there.
                return Ok(None);
            }
            let outcome = self.open_town_arrest(arrest, Some(game_dir))?;
            self.push_diagnostic(format!(
                "Guard NPC slot {npc_slot} catches the party; arrest opened."
            ));
            return Ok(Some(outcome));
        }
        if live_sprite >= TOWN_NPC_CONTACT_CONFLICT_ACTOR_FIRST {
            self.mark_visibility_dirty();
            self.emit_message_line(TOWN_NPC_CONTACT_ATTACKED_LINE);
            // `town-mode.md §14`: the NPC-conflict chain, "entered both from
            // A-Attack and from post-action cleanup, ... hands the target
            // NPC's linked active-object slot to the same terrain-combat
            // entry the overworld uses". The trigger is stamped the way the
            // A-Attack entry stamps it.
            if let Some((object_slot, object)) = linked {
                let trigger = ActiveObject {
                    type_byte,
                    tile: type_byte,
                    ..object
                };
                if game_dir.join(BRIT_CBT_FILE).exists()
                    && terrain_combat_base_class(trigger).is_some()
                {
                    self.pending_town_conflict = Some(PendingTownConflict {
                        scene_byte: scene.byte,
                        floor,
                        npc_slot,
                        type_byte,
                        awaiting_floor_reload: false,
                        // `town-mode.md §7` step 4 (issue #298 Q40): a fight
                        // raised by post-action cleanup "comes after its
                        // turn's clock minute, underfoot pass and both
                        // walkers, and none of them runs again after it".
                        owes_turn_epilogue: false,
                    });
                    let hostile_terrain = self.grid[object.y * TOWN_GRID_SIDE + object.x];
                    let _setup_report = self
                        .enter_terrain_combat_from_object_in_scene_with_terrain(
                            game_dir,
                            WorldPlane::Britannia,
                            object_slot,
                            trigger,
                            scene.byte,
                            hostile_terrain,
                        )?;
                }
            }
            self.push_diagnostic(format!(
                "NPC slot {npc_slot} (sprite 0x{live_sprite:02X}) contact enters NPC conflict."
            ));
            return Ok(Some(MoveOutcome::Used));
        }
        self.clear_town_conflict_npc_slot(scene, npc_slot, type_byte);
        self.push_diagnostic(format!(
            "NPC slot {npc_slot} (sprite 0x{live_sprite:02X}) contact clears the NPC slot."
        ));
        Ok(Some(MoveOutcome::Used))
    }

    pub fn resolve_town_arrest_prompt(
        &mut self,
        key: char,
        game_dir: &Path,
    ) -> io::Result<Option<MoveOutcome>> {
        let Some(prompt) = self.pending_town_arrest else {
            return Ok(None);
        };
        match key.to_ascii_lowercase() {
            'y' => {
                // `town-mode.md §1123`: the `:` answer row "echoes `Yes`
                // or `No!`", so the accepted key completes the open row
                // before the outcome prints on the next one.
                self.commit_prompt_reply(":", TOWN_ARREST_YES_REPLY);
                // `town-mode.md §14` (u5-spec#300 Q33): `:Yes`, then a blank
                // row before the knockout line - the echo ends its row.
                self.message_row_open_mid_line = false;
                self.pending_town_arrest = None;
                if prompt.scene_byte == BLACKTHORN_CAPTIVE_CELL_SCENE {
                    return self.begin_blackthorn_audience_capture(game_dir);
                }
                self.apply_town_arrest_surrender(game_dir)
            }
            'n' => {
                // `town-mode.md §14` (u5-spec#300 Q7, R592): "The refusal
                // echo is `No`, not `No!`".
                self.commit_prompt_reply(":", crate::commands::TOWN_ARREST_REFUSAL_NO_REPLY);
                self.message_row_open_mid_line = false;
                self.pending_town_arrest = None;
                let scene = Scene::new(prompt.scene_byte)?;
                // "the colon row completes with `No`, then a blank row,
                // `"Then defend thyself, rogue!"` and the alarm sweep".
                self.emit_message_line(format!(
                    "\n{}\n",
                    crate::commands::TOWN_ARREST_REFUSAL_CHALLENGE
                ));
                let (pursued, fled) =
                    self.town_alarm_sweep(scene, prompt.floor, Some(prompt.npc_slot));
                self.push_diagnostic(format!(
                    "Refused surrender; alarm raised ({pursued} pursuing, {fled} fleeing)."
                ));
                // "Post-action cleanup then takes its conflict follow-up:
                // when the linked object's actor byte is at least `0x40`
                // (a guard's always is) it prints `Attacked!` with its
                // leading line feed and enters the NPC-conflict chain, an
                // ordinary town arena fight" (R591 recorded the omission).
                self.start_town_arrest_refusal_conflict(scene, prompt.floor, prompt.npc_slot, game_dir)?;
                Ok(Some(MoveOutcome::Used))
            }
            // `town-mode.md §14` prints the challenge once; an unaccepted
            // key is "rejected by the yes/no wait", which "continues to wait
            // for a valid answer" (`npc-schedules.md §9.2`). Re-assigning
            // the challenge to the slot printed the whole block again under
            // the open `:` row on every stray key (playtest N26).
            _ => Ok(Some(MoveOutcome::PromptDeclined)),
        }
    }

    pub fn apply_town_arrest_surrender(
        &mut self,
        game_dir: &Path,
    ) -> io::Result<Option<MoveOutcome>> {
        let scene = Scene::new(TOWN_ARREST_JAIL_SCENE)?;
        let floor = TOWN_ARREST_JAIL_FLOOR as i8;
        let (grid, beacon_sources) =
            load_town_runtime_floor_with_beacon_sources(game_dir, scene, floor, self.clock.hour)?;
        self.grid = grid;
        // `visibility.md §12.6`: a new location floor is fresh map setup —
        // clear both beacon positions and re-record up to two bright-light
        // hits. Harvested from the RAW floor, because the runtime
        // normalisation pass scrubs the marker byte the beacon looks for.
        self.light_beacon.sources = beacon_sources;
        self.natural_moongate_live_cells.clear();
        self.area = Area::Town { scene, floor };
        self.player.x = TOWN_ARREST_JAIL_X as usize;
        self.player.y = TOWN_ARREST_JAIL_Y as usize;
        self.player.transport = TransportState::Foot;
        self.clear_town_floor_reload_door_state();
        let tlk = parse_tlk(&game_dir.join(format!("{}.TLK", scene.family.stem())))?;
        let npc_slots = parse_npc_block(game_dir, scene, &tlk)?;
        self.load_scheduled_npcs(&npc_slots);
        let _ = self.restore_resident_shadowlord_after_floor_reload();
        self.sync_player_object();
        self.mark_visibility_dirty();
        self.advance_town_arrest_wait_to_morning();
        // `moons.md §3` (issue #190; wake time corrected 2026-10-25,
        // u5-spec#300 Q33, `RETRACTIONS.md` R617): "The town arrest jail
        // relocation refreshes. The relocation arm - the one that moves the
        // party to the Yew jail, runs the clock forward into the 08:00 hour
        // in repeated twenty-minute cleanup calls" - so the wake is 08:00 to
        // 08:19, which the loop above already does by testing the hour
        // before each step - "and sets the floor byte to the entry floor - ends by running the
        // town entry pass, so it reaches the floor loader and repaints
        // unconditionally; because it clears the floor byte first, that
        // repaint takes the **normal** arm rather than the erase arm."
        //
        // The entry pass runs at the *end*, so the refresh is here rather
        // than beside the floor load above: the twenty-minute burst can
        // cross a day rollover, and the pair the pass caches is the one for
        // the day the party wakes on.
        self.refresh_cached_moon_glyphs_at_scene_entry();
        self.push_diagnostic(format!(
            "Surrendered to the guards; jailed in {} at ({}, {}) until {:02}:00.",
            scene.key(),
            self.player.x,
            self.player.y,
            self.clock.hour
        ));
        // `town-mode.md §14` (u5-spec#300 Q7, Q33): "the colon row
        // completes with `Yes`; a blank row and `The guard strikes thee
        // unconscious!`; the fade; a blank row and `Thou dost awaken
        // to...`". Each literal is a line feed, the text, a line feed.
        self.message = format!(
            "\n{TOWN_ARREST_KNOCKOUT_MESSAGE}\n\n{}",
            crate::commands::TOWN_ARREST_AWAKENING_MESSAGE
        );
        Ok(Some(MoveOutcome::Transition(
            AreaTransition::EnteredLocation(scene),
        )))
    }

    /// `town-mode.md §14` (u5-spec#300 Q7): the refusal's conflict
    /// follow-up. The fight's target is "the roster index held in the
    /// retained contact state"; when its linked actor byte is at least
    /// `0x40`, `Attacked!` prints and the ordinary town arena fight starts,
    /// as the contact arm of post-action cleanup does.
    fn start_town_arrest_refusal_conflict(
        &mut self,
        scene: Scene,
        floor: i8,
        npc_slot: usize,
        game_dir: &Path,
    ) -> io::Result<()> {
        let Some((type_byte, object_slot, object)) = self.npcs.iter().find_map(|npc| {
            (npc.slot == npc_slot)
                .then_some(npc.active_object)
                .flatten()
                .and_then(|slot| {
                    self.active_objects
                        .get(slot)
                        .copied()
                        .filter(|object| !object.is_empty())
                        .map(|object| (npc.type_byte, slot, object))
                })
        }) else {
            return Ok(());
        };
        if object.type_byte < TOWN_NPC_CONTACT_CONFLICT_ACTOR_FIRST {
            return Ok(());
        }
        self.mark_visibility_dirty();
        self.emit_message_line(TOWN_NPC_CONTACT_ATTACKED_LINE);
        let trigger = ActiveObject {
            type_byte,
            tile: type_byte,
            ..object
        };
        if game_dir.join(BRIT_CBT_FILE).exists() && terrain_combat_base_class(trigger).is_some() {
            self.pending_town_conflict = Some(PendingTownConflict {
                scene_byte: scene.byte,
                floor,
                npc_slot,
                type_byte,
                awaiting_floor_reload: false,
                // The turn was already spent by the epilogue that reached
                // cleanup; nothing of it runs again after the fight.
                owes_turn_epilogue: false,
            });
            let hostile_terrain = self.grid[object.y * TOWN_GRID_SIDE + object.x];
            let _setup_report = self.enter_terrain_combat_from_object_in_scene_with_terrain(
                game_dir,
                WorldPlane::Britannia,
                object_slot,
                trigger,
                scene.byte,
                hostile_terrain,
            )?;
        }
        Ok(())
    }

    /// The arrest wait-to-morning loop (`time.md §10`: "advances time in
    /// repeated twenty-minute cleanup calls until the hour byte reaches
    /// 08:00"). Returns the number of cleanup calls made.
    ///
    /// `time.md §12` lists this loop as a twenty-minute *cleanup* caller, and
    /// the party status pass census is "exactly four places: the overworld
    /// per-turn epilogue, the town per-turn underfoot handler, the dungeon
    /// post-action handler, and the town-bed rest loop's ten-minute step".
    /// The arrest loop is not one of them, so the wait takes no poison
    /// points, eats no meal and leaves the turn-step counter alone - that
    /// counter's only increment is the upkeep pass (`formats/saved-gam.md`
    /// `0x02E5`) (playtest W1).
    ///
    /// Nor is it a schedule-processor caller: `npc-schedules.md §5` names
    /// only the town turn loop and the H-Hole-Up burst, so the wait leaves
    /// every NPC where the jail load placed it. Running the scheduler here
    /// walked Yew's guards off their posts overnight (playtest K05).
    pub(crate) fn advance_town_arrest_wait_to_morning(&mut self) -> u16 {
        let mut cleanup_ticks = 0;
        while !town_arrest_release_loop_done(self.clock.hour) && cleanup_ticks < 96 {
            self.advance_turn_with_minutes_policy_and_schedules(
                TOWN_ARREST_CLEANUP_INCREMENT_MINUTES,
                false,
                true,
                false,
                false,
                false,
            );
            cleanup_ticks += 1;
        }
        cleanup_ticks
    }

    /// `town-mode.md §14`: "The arrest sequence itself branches on the
    /// current location **before it prints anything**. Inside Lord
    /// Blackthorn's Castle ... it plays the Blackthorn audience/capture
    /// cinematic ... In every other location it prints the arrest
    /// challenge." Every caller that reaches an arrest - the guard-event
    /// contact and both refused guard demands, which `blackthorn.md §7a`
    /// hands to "the caller ... under `town-mode.md` Section 14" - goes
    /// through here, so the branch happens once and in one place.
    ///
    /// `game_dir` is `None` on the dialogue-map Talk paths, which carry no
    /// assets and so cannot run the cinematic; those keep the surrender
    /// prompt.
    pub(crate) fn open_town_arrest(
        &mut self,
        arrest: TownArrestPrompt,
        game_dir: Option<&Path>,
    ) -> io::Result<MoveOutcome> {
        if arrest.scene_byte == crate::blackthorn::BLACKTHORN_CAPTIVE_CELL_SCENE
            && let Some(game_dir) = game_dir
            && self.party_capability() != PartyCapability::Defeated
        {
            return Ok(self
                .begin_blackthorn_audience_capture(game_dir)?
                .unwrap_or(MoveOutcome::Used));
        }
        // `blackthorn.md §3`: "If the party reaches that helper there with
        // nobody able to act and nobody asleep, the helper returns silently,
        // without the audience or a surrender question. The exploration
        // loop's separate defeat check still owns rescue." The ordinary
        // surrender fallback in this palace case is withdrawn (R442).
        if arrest.scene_byte == crate::blackthorn::BLACKTHORN_CAPTIVE_CELL_SCENE
            && self.party_capability() == PartyCapability::Defeated
        {
            return Ok(MoveOutcome::Used);
        }
        self.pending_town_arrest = Some(arrest);
        // The challenge opens on its own row: it is a fresh two-line
        // exchange, not a tail appended to whatever the contact
        // already printed.
        //
        // When the turn's own result is already in the transcript (the
        // step's `Blocked!` was flushed before the post-action cleanup
        // reached the guard), re-prefixing the slot would flush that line a
        // second time - the doubled `Blocked!` above the challenge in
        // playtest Q03. Print the challenge beneath it instead. The row
        // state is set explicitly to what the slot path below leaves (it
        // never touches the flag), so the `:` answer row lays out the same
        // either way.
        if !self.message.is_empty() && self.message == self.message_flushed {
            let row_open = self.message_row_open_mid_line;
            self.emit_message_line(TOWN_ARREST_SURRENDER_PROMPT);
            self.message_row_open_mid_line = row_open;
            return Ok(MoveOutcome::Used);
        }
        self.message = if self.message.is_empty() {
            TOWN_ARREST_SURRENDER_PROMPT.to_string()
        } else {
            format!("{}\n{TOWN_ARREST_SURRENDER_PROMPT}", self.message)
        };
        Ok(MoveOutcome::Used)
    }

    pub fn begin_blackthorn_audience_capture(
        &mut self,
        game_dir: &Path,
    ) -> io::Result<Option<MoveOutcome>> {
        self.pending_town_arrest = None;
        self.blackthorn_audience_map = None;
        self.blackthorn_capture = None;
        // `blackthorn.md §3`, capture narration timeline, step 1 (issue #304
        // Q6, `RETRACTIONS.md` R630): "One world tick repaints the view.
        // Print `\nThou art subdued and blindfolded!`". Setup step 1 prints
        // only this line; the rest of the narration follows the shrine
        // selection, and an abandoned audience prints nothing more. On the
        // ordinary path the line is the narration's first staged beat, at its
        // measured "+0.12 s" (`stage_blackthorn_capture_narration`); an
        // abandoned audience prints it here, before its handoff.

        // `moons.md §3` (issue #190, `RETRACTIONS.md` R375): "The
        // Blackthorn audience cutscene repaints once, not twice. The
        // audience routine contains two repaint calls. The first, on
        // entry, always paints, and always takes the erase arm: the party
        // level is forced below surface immediately before it and the
        // scene is still Blackthorn's castle."
        //
        // This is that first call. R375 is the reason there is no matching
        // call in the routine's tail on the ordinary cutscene path: "on the
        // ordinary cutscene path the scene byte has been forced out of the
        // renderer's range earlier in the routine and is not restored until
        // after that second call, so the renderer returns at its first gate
        // having painted nothing and cached nothing ... An implementation
        // that wires both of the routine's own calls paints an extra
        // erase-arm frame the original never paints." The real
        // post-cutscene refresh comes from the caller instead - the town
        // entry pass, whose floor loader repaints (`reload_town_floor`).
        //
        // The below-surface level force is presentation-only here: this
        // engine paints no strip during the cutscene, and which arm the
        // painter would take does not change the cache write, which
        // precedes both erase-arm tests (§3).
        self.refresh_cached_moon_glyphs_at_scene_entry();

        // `blackthorn.md §3` step 2: "Select which shrine the interrogation
        // will demand a mantra for: scan the eight shrine ruin flags in
        // shrine order and take the first whose flag is *exactly* clear —
        // never ruined and never restored. If every flag is non-zero the
        // whole audience is abandoned, and the first narration line is the
        // only one printed." Timeline step 2: "If no shrine qualifies, the
        // narration ends here" - before step 3's clear of the actor table.
        let Some(shrine_index) = self.blackthorn_selected_shrine() else {
            self.emit_blackthorn_capture_first_line();
            // `moons.md §3` (R375): the routine's *second* repaint "paints
            // only on the routine's early-exit path". This is one of the
            // two early exits, so it is wired here and nowhere on the
            // ordinary cutscene path.
            self.refresh_cached_moon_glyphs_at_scene_entry();
            self.push_diagnostic(
                "Blackthorn audience found no un-ruined shrine to interrogate.".to_string(),
            );
            let outcome = self.apply_blackthorn_captive_cell_handoff(game_dir, "")?;
            return Ok(Some(outcome));
        };

        // `blackthorn.md §3`: the setup "counts the active party members
        // that are still eligible for the challenge". Eligibility is
        // liveness only - the §3 withdrawal is explicit that "**There is
        // no per-member jail flag.**"
        if self.blackthorn_eligible_party_member_count() == 0 {
            self.emit_blackthorn_capture_first_line();
            // `moons.md §3` (R375): the routine's second repaint, on the
            // other early-exit path. See the shrine-scan exit above.
            self.refresh_cached_moon_glyphs_at_scene_entry();
            self.clear_non_player_active_objects();
            self.mark_visibility_dirty();
            self.push_diagnostic("Blackthorn audience found no eligible party member.".to_string());
            let outcome = self.apply_blackthorn_captive_cell_handoff(game_dir, "")?;
            return Ok(Some(outcome));
        }

        // The map is read now so a missing asset fails at the arrest, but it
        // goes on screen only at timeline step 8 (setup steps 3 to 6).
        let map = load_miscmaps_cutscene_map(game_dir, BLACKTHORN_AUDIENCE_CUTSCENE_MAP_RECORD)?;
        self.blackthorn_capture = Some(crate::blackthorn::BlackthornCaptureNarration {
            map,
            script: None,
        });
        self.stage_blackthorn_capture_narration();

        // The audience's first key read is setup step 8's greeting
        // acknowledgement. Until the narration's last beat has printed the
        // greeting, `cinematic_hold_owns_input` holds every key in the queue:
        // "None of the narration's steps polls the keyboard ... Keys typed
        // during the narration therefore wait in the keyboard queue in the
        // order typed" (`blackthorn.md §3`, "Keys typed during the capture
        // narration", issue #304 Q7). Under Buffer Off the acknowledgement
        // uses one and the rest are discarded (`drain_buffered_hold_key`).
        let mut challenge =
            crate::blackthorn_session::BlackthornChallenge::for_shrine(shrine_index as u8);
        challenge.await_greeting_acknowledgement();
        self.active_blackthorn = Some(challenge);
        Ok(Some(MoveOutcome::Used))
    }

    /// `blackthorn.md §3`, timeline step 1's line, printed at once. Its last
    /// row, `and blindfolded!`, fills all sixteen cells, so the cursor is
    /// already at column 0 of the next row: "A line feed arriving from any
    /// **later** call is unconditional and always costs a row"
    /// (`text-output.md §6`). The next line's two feeds therefore leave two
    /// blank rows under it, as the stock capture shows (`bt-capture-pacing`,
    /// every beat from `s01`).
    fn emit_blackthorn_capture_first_line(&mut self) {
        let line = crate::blackthorn::BLACKTHORN_CAPTURE_SUBDUED_LINE;
        self.emit_message_line(line);
        self.message_row_open_mid_line = crate::blackthorn::presentation_print_leaves_row_open(line);
    }

    /// Stage `blackthorn.md §3`'s capture narration timeline, steps 1 and 3
    /// to 19 (issue #304 Q6): every hold, sting, line and placement in order,
    /// with the greeting of setup step 8 last.
    ///
    /// The holds are the published counts paced on `karma.md §7`'s lattice
    /// ([`crate::blackthorn::blackthorn_capture_release_milliticks`]),
    /// anchored at the arrest. Holding each count as whole ticks of wall
    /// clock lost the edges the original loses to its printing: the dragged-
    /// away, chained and `Footsteps!` lines each push the next request one
    /// edge later, and the first line itself comes "+0.12 s" after the echo.
    /// The engine ran about 0.3 s ahead of the stock game by the guards
    /// (`bt-audience`/`ask1`, `bt-capture-pacing` 2026-10-27).
    fn stage_blackthorn_capture_narration(&mut self) {
        use crate::blackthorn::BlackthornCaptureCue;
        // Step 7: "With sound on, eighteen repetitions ...; with sound off,
        // three." `music_enabled` is the game's sound setting (Ctrl-S). The
        // engine has no cinematic-animation setting; it is always on.
        let timeline = crate::blackthorn::blackthorn_capture_timeline(self.music_enabled, true);
        // Steps 18 and 19, setup step 8 (u5-spec#298 Q8): after the quiet
        // 8-tick pause, the greeting to roster record 0 - "the Avatar's
        // record, never a member chosen any other way" - ending on one
        // trailing space and no line feed, then "**Wait for an
        // acknowledgement key.**"
        let greeting =
            crate::blackthorn::blackthorn_greeting_line(&self.blackthorn_roster_record_zero_name());
        // `audio.md §8.6`: "It blocks, and the view does not redraw while it
        // plays." The envelope's hold is the program's own length.
        let envelope = crate::blackthorn::secs_milliticks(
            SoundEffect::BlackthornThroneApproachEnvelope
                .program(&mut crate::audio::RumbleJitter::default())
                .duration(true)
                .as_secs_f64(),
        );
        let text_of = |beat: &crate::blackthorn::BlackthornCaptureBeat| match beat.cue {
            Some(BlackthornCaptureCue::Greeting) => Some(greeting.clone()),
            _ => beat.text.map(str::to_string),
        };
        let releases =
            crate::blackthorn::blackthorn_capture_release_milliticks(&timeline, 0, envelope, &text_of);
        self.stage_blackthorn_capture_beats(&timeline, &releases, text_of);
    }

    /// Push staged beats released at the given lattice times (thousandths of
    /// a tick from the presentation's origin, which is now).
    fn stage_blackthorn_capture_beats(
        &mut self,
        beats: &[crate::blackthorn::BlackthornCaptureBeat],
        releases: &[u32],
        text_of: impl Fn(&crate::blackthorn::BlackthornCaptureBeat) -> Option<String>,
    ) {
        use crate::narration::NarrationCue;
        let mut previous = 0;
        for (beat, &release) in beats.iter().zip(releases) {
            let secs = crate::blackthorn::milliticks_secs(release - previous);
            previous = release;
            let text = text_of(beat).unwrap_or_default();
            // Every capture beat carries a cue, so its release can state the
            // row a line leaves (`finish_narration_cue_after_text`).
            let cue = beat.cue.unwrap_or(crate::blackthorn::BlackthornCaptureCue::Line);
            self.staged_narration
                .push_secs_with_cue(secs, NarrationCue::BlackthornCapture(cue), text);
        }
    }

    /// Stage setup step 8's guard-release route and step 9's preamble
    /// (`blackthorn.md §3`, §6.3 "Guard release route") on the pacing rule's
    /// lattice, anchored at the greeting's key with the release order
    /// already printed there.
    fn stage_blackthorn_guard_release_route(&mut self, release_line: &str, preamble: Option<String>) {
        use crate::blackthorn::BlackthornCaptureCue;
        self.blackthorn_capture = Some(crate::blackthorn::BlackthornCaptureNarration::default());
        let route = crate::blackthorn::blackthorn_guard_release_timeline();
        let text_of = |beat: &crate::blackthorn::BlackthornCaptureBeat| match beat.cue {
            Some(BlackthornCaptureCue::ReleaseRouteEnd) => preamble.clone(),
            _ => None,
        };
        let start = crate::blackthorn::presentation_printed_rows(release_line)
            * crate::blackthorn::PRESENTATION_ROW_PRINT_MILLITICKS;
        let releases = crate::blackthorn::blackthorn_capture_release_milliticks(&route, start, 0, &text_of);
        self.stage_blackthorn_capture_beats(&route, &releases, text_of);
    }

    /// Run one staged narration cue as its beat is released, before the
    /// beat's text prints.
    pub(crate) fn release_narration_cue(&mut self, cue: crate::narration::NarrationCue) {
        match cue {
            crate::narration::NarrationCue::BlackthornCapture(cue) => {
                self.release_blackthorn_capture_cue(cue);
            }
        }
    }

    /// `blackthorn.md §3`, capture narration timeline (issue #304 Q6).
    fn release_blackthorn_capture_cue(&mut self, cue: crate::blackthorn::BlackthornCaptureCue) {
        use crate::blackthorn::BlackthornCaptureCue;
        match cue {
            // Step 4: "Fill the map viewport black (the blindfold)".
            BlackthornCaptureCue::Blindfold => {
                self.cutscene_black_viewport = true;
            }
            // Steps 5, 7 and 13: the shared two-part sting (`audio.md §8.6`,
            // the timer-paced and footstep rows).
            BlackthornCaptureCue::Sting => {
                self.emit_sound_effect(SoundEffect::BlackthornMovementStinger);
            }
            // Step 8: "Clear the actor table, load the map and the messages,
            // and place the party (setup steps 3 to 6)." Step 9: the quiet
            // 16-tick pause's "first world tick paints the cutscene map with
            // the manacled prisoners", which ends the blindfold fill.
            BlackthornCaptureCue::RevealPrisoners => {
                self.clear_non_player_active_objects();
                // Without the asset, the same blank grid the script VM falls
                // back to, so the cutscene state is still the audience's.
                self.blackthorn_audience_map = Some(
                    self.blackthorn_capture
                        .as_mut()
                        .and_then(|capture| capture.map.take())
                        .unwrap_or_else(|| MiscmapsCutsceneMap {
                            record_index: BLACKTHORN_AUDIENCE_CUTSCENE_MAP_RECORD,
                            tiles: vec![0; MISCMAPS_CUTSCENE_ROWS * MISCMAPS_CUTSCENE_VISIBLE_COLUMNS],
                        }),
                );
                self.install_blackthorn_audience_party();
                self.cutscene_black_viewport = false;
                self.mark_visibility_dirty();
            }
            // Step 14: "Place the two guards at (4,10) and (6,10). They
            // first show at the next redraw."
            BlackthornCaptureCue::PlaceGuards => {
                self.install_blackthorn_audience_guards();
            }
            // Step 15: the guard-approach script. It is run whole on its
            // first sting, and each sting then shows the actors as the script
            // left them at that sting.
            BlackthornCaptureCue::GuardApproachSting(index) => {
                if index == 0 {
                    let mut vm = self.blackthorn_cutscene_vm_from_audience_state();
                    vm.run(blackthorn_cutscene_beat_commands(
                        BlackthornCutsceneBeat::AudienceThroneApproach,
                    ));
                    self.push_diagnostic(format!(
                        "Blackthorn audience opened: the approach advanced {} world ticks.",
                        vm.world_ticks,
                    ));
                    if let Some(capture) = self.blackthorn_capture.as_mut() {
                        capture.script = Some(vm);
                    }
                }
                let frame = self
                    .blackthorn_capture
                    .as_ref()
                    .and_then(|capture| capture.script.as_ref())
                    .and_then(|vm| vm.actors_at_each_sting.get(usize::from(index)).copied());
                if let Some(actors) = frame {
                    self.apply_blackthorn_guard_frame(&actors);
                }
                self.emit_sound_effect(SoundEffect::BlackthornMovementStinger);
            }
            // Step 16: one blocking software envelope; the hold that follows
            // this beat is its length.
            BlackthornCaptureCue::ThroneEnvelope => {
                if let Some(vm) = self
                    .blackthorn_capture
                    .as_mut()
                    .and_then(|capture| capture.script.take())
                {
                    self.apply_blackthorn_cutscene_vm_to_audience_state(&vm);
                }
                self.emit_sound_effect(SoundEffect::BlackthornThroneApproachEnvelope);
            }
            // Step 17: "Hide slot 8 at (5,5) and reveal seated Blackthorn
            // there with the 256-pixel transition ... Then assign actor byte
            // `0x78`." The engine shows the seated figure when the
            // transition's hold ends.
            BlackthornCaptureCue::ThroneRevealed => {
                let mut vm = self.blackthorn_cutscene_vm_from_audience_state();
                vm.run(blackthorn_cutscene_beat_commands(
                    BlackthornCutsceneBeat::AudienceThroneReveal,
                ));
                self.apply_blackthorn_cutscene_vm_to_audience_state(&vm);
            }
            // Step 19: see [`Self::finish_narration_cue_after_text`].
            BlackthornCaptureCue::Greeting | BlackthornCaptureCue::Line => {}
            // Setup step 8's guard-release route (§6.3), run whole on its
            // first step like the approach, each step then showing the
            // acting guard where the script had it at that step's sting.
            BlackthornCaptureCue::GuardReleaseStep(index) => {
                if index == 0 {
                    let mut vm = self.blackthorn_cutscene_vm_from_audience_state();
                    vm.run(blackthorn_cutscene_beat_commands(
                        BlackthornCutsceneBeat::GuardReleaseRoute,
                    ));
                    self.push_diagnostic(format!(
                        "Blackthorn audience: the guard release advanced {} world ticks.",
                        vm.world_ticks,
                    ));
                    if let Some(capture) = self.blackthorn_capture.as_mut() {
                        capture.script = Some(vm);
                    }
                }
                let frame = self
                    .blackthorn_capture
                    .as_ref()
                    .and_then(|capture| capture.script.as_ref())
                    .and_then(|vm| vm.actors_at_each_sting.get(usize::from(index)).copied());
                if let Some(actors) = frame {
                    self.apply_blackthorn_guard_frame(&actors);
                }
                self.emit_sound_effect(SoundEffect::BlackthornMovementStinger);
            }
            // Setup step 9: the route's last redraw is over; the script's
            // end state stands, and the preamble prints.
            BlackthornCaptureCue::ReleaseRouteEnd => {
                if let Some(vm) = self
                    .blackthorn_capture
                    .as_mut()
                    .and_then(|capture| capture.script.take())
                {
                    self.apply_blackthorn_cutscene_vm_to_audience_state(&vm);
                }
                self.blackthorn_capture = None;
            }
        }
    }

    /// The part of a cue that follows its beat's text.
    pub(crate) fn finish_narration_cue_after_text(
        &mut self,
        cue: crate::narration::NarrationCue,
        text: &str,
    ) {
        match cue {
            // Setup step 8: the greeting ends on "one trailing space and no
            // line feed", so its row stays open for the acknowledgement's
            // print; stated here rather than read off the text. The
            // narration is over.
            crate::narration::NarrationCue::BlackthornCapture(
                crate::blackthorn::BlackthornCaptureCue::Greeting,
            ) => {
                self.open_message_row();
                self.blackthorn_capture = None;
            }
            // The preamble record ends on its trailing space; the audience's
            // second acknowledgement reads with that row open (§4.1).
            crate::narration::NarrationCue::BlackthornCapture(
                crate::blackthorn::BlackthornCaptureCue::ReleaseRouteEnd,
            ) => {
                self.open_message_row();
            }
            // Any other narration line: open unless it ends in a feed or on a
            // row-filling last row - `Thou art subdued and blindfolded!`
            // ends on `and blindfolded!`, all sixteen cells (`text-output.md
            // §6`).
            crate::narration::NarrationCue::BlackthornCapture(_) => {
                self.message_row_open_mid_line =
                    crate::blackthorn::presentation_print_leaves_row_open(text);
            }
        }
    }

    /// Show the two guards where the guard-approach script had them at one
    /// of its stings.
    fn apply_blackthorn_guard_frame(
        &mut self,
        actors: &[Option<BlackthornCutsceneActorState>; crate::blackthorn::BLACKTHORN_CUTSCENE_ACTOR_SLOT_COUNT],
    ) {
        for guard in [BlackthornCutsceneActor::LeftGuard, BlackthornCutsceneActor::RightGuard] {
            let slot = guard.slot_index() as usize;
            let (Some(state), Some(object)) = (actors[slot], self.active_objects.get_mut(slot)) else {
                continue;
            };
            object.x = state.x;
            object.y = state.y;
        }
        self.mark_visibility_dirty();
    }

    /// Whether `blackthorn.md §3`'s capture narration is still staged.
    pub fn blackthorn_capture_narration_active(&self) -> bool {
        self.blackthorn_capture.is_some()
    }

    /// `blackthorn.md §4.1`: the audience preamble is record
    /// [`crate::MISCMSG_BLACKTHORN_AUDIENCE_PREAMBLE`], "the Wait/Avatarhood
    /// speech". Its own leading line feeds and trailing space are authored,
    /// so the record is used exactly as it reads - §4.1 asks callers to
    /// "preserve the selected record's wording and spacing".
    pub fn blackthorn_audience_opening_text(&self, game_dir: &Path) -> io::Result<Option<String>> {
        let Some(messages) = load_misc_messages(game_dir)? else {
            return Ok(None);
        };
        Ok(messages
            .record(crate::MISCMSG_BLACKTHORN_AUDIENCE_PREAMBLE)
            .filter(|record| !record.trim().is_empty())
            .map(str::to_string))
    }

    /// The three already-resolved arms print nothing: `blackthorn.md §4.1`
    /// publishes no line for them, and the sentence the engine used to print
    /// was a status report about its own state machine.
    fn blackthorn_resolved_handoff(
        &mut self,
        game_dir: &Path,
        note: &str,
    ) -> io::Result<MoveOutcome> {
        self.push_diagnostic(note.to_string());
        self.apply_blackthorn_captive_cell_handoff(game_dir, "")
    }

    /// `blackthorn.md §5` step 5: print a resolved outcome and hold it on
    /// screen until the player acknowledges it. The captive-cell handoff runs
    /// on that key, not before - the engine ran it immediately, so the frame
    /// carried a live command row where the stock game's was still blank.
    fn hold_blackthorn_closing(
        &mut self,
        challenge: crate::blackthorn_session::BlackthornChallenge,
        line: String,
    ) -> io::Result<MoveOutcome> {
        self.hold_blackthorn_closing_page(challenge, line, false)
    }

    /// Print a closing record and wait for its acknowledgement.
    ///
    /// No line feed follows here. `blackthorn.md §5`'s executions "finish
    /// with one line feed"; the correct-answer arm's is printed after the
    /// blade hold's roster edit by [`Self::release_blackthorn_execution_hold`]
    /// (the blank row seen during the hold is record `5`'s own row-filling
    /// wrap, u5-spec#299 Q9), and the wrong-answer arm's comes after record
    /// `6`. The two branches that
    /// execute nobody, records `9` and `10`, end on "acknowledgement and the
    /// closing scene beat" with no such row: `bt-audience`'s `ask3` beat has
    /// the next command echo directly under `thee!"`.
    fn hold_blackthorn_closing_page(
        &mut self,
        mut challenge: crate::blackthorn_session::BlackthornChallenge,
        line: String,
        epilogue: bool,
    ) -> io::Result<MoveOutcome> {
        challenge.await_closing_acknowledgement(epilogue);
        self.active_blackthorn = Some(challenge);
        self.emit_message_line(line.clone());
        self.message = line;
        Ok(MoveOutcome::PromptDeclined)
    }

    /// `blackthorn.md §4.2` "Where the cursor is left": "Records `9` and `10`
    /// both end with a closing double quote followed by one trailing space,
    /// and neither contains a line feed. The cursor is therefore left
    /// mid-row, one cell past that trailing space, and it stays there through
    /// the beat, through the handoff, and into the next scene."
    ///
    /// The print set the cursor state but not the transcript row's, so the
    /// window derived `text-output.md §10.4`'s blank under the record and the
    /// captive cell's first prompt sat a row low. Measured on `bt-password`
    /// (20261024-001911, beat `onward`): the stock reads `thee!"` with the
    /// cell's marker row directly under it. Stated here, per caller, rather
    /// than inferred from the record's text.
    fn leave_solo_closing_record_row_open(&mut self) {
        self.message_row_open_mid_line = true;
        self.leave_last_transcript_row_open();
    }

    /// `blackthorn.md §4.1`'s reaction records. The measured constants are
    /// the fallbacks for the paths that run without `MISCMSG.DAT`.
    pub fn blackthorn_reaction_text(
        &self,
        game_dir: &Path,
        record: usize,
        fallback: &str,
    ) -> io::Result<String> {
        Ok(load_misc_messages(game_dir)?
            .and_then(|messages| messages.record(record).map(str::to_string))
            .filter(|text| !text.trim().is_empty())
            .unwrap_or_else(|| fallback.to_string()))
    }

    /// `blackthorn.md §4.1`: the demand for one prompt ordinal. Records
    /// `0..=3` hold the four escalating wordings; the first three end on the
    /// virtue's place and take the virtue name plus the question-mark and
    /// closing quote, and the fourth "has an opening quotation mark but **no
    /// closing quotation mark**, and its caller adds none".
    ///
    /// The fallbacks are the same four wordings, measured, for the paths that
    /// run without `MISCMSG.DAT`.
    pub fn blackthorn_demand_text(
        &self,
        game_dir: &Path,
        ordinal: u8,
        virtue: &str,
    ) -> io::Result<String> {
        let index = usize::from(ordinal).min(*crate::MISCMSG_BLACKTHORN_DEMAND_RANGE.end());
        let record = load_misc_messages(game_dir)?
            .and_then(|messages| messages.record(index).map(str::to_string))
            .filter(|record| !record.trim().is_empty());
        let last = *crate::MISCMSG_BLACKTHORN_DEMAND_RANGE.end();
        Ok(match record {
            Some(record) if index == last => record,
            Some(record) => format!("{record}{virtue}?\""),
            None => blackthorn_demand_fallback(index).replace("{}", virtue),
        })
    }

    /// Setup step 6 whole: the party, then the two guards. The narration
    /// splits these (`blackthorn.md §3`, capture narration timeline, steps 8
    /// and 14); this is for callers that set the cutscene up directly.
    pub fn install_blackthorn_audience_actors(&mut self) {
        self.install_blackthorn_audience_party();
        self.install_blackthorn_audience_guards();
    }

    /// Setup step 6's party placement. The seated-Blackthorn slot (slot 8)
    /// "is not set up in this step: it still holds only the zeroed record
    /// left by the clear of step 3" (`blackthorn.md §3`, issue #304 Q6,
    /// `RETRACTIONS.md` R631).
    pub fn install_blackthorn_audience_party(&mut self) {
        if self.active_objects.len() < OOL_SLOTS {
            self.active_objects.resize(OOL_SLOTS, ActiveObject::empty());
        }
        // `blackthorn.md §3` "Party placement (step 6)" (u5-spec#299 Q11):
        // "Setup places the first *n* roster records, in roster order, into
        // cutscene actor slots 0 to *n*-1", *n* the nondead count. "The bound
        // is a count, not a filter": a Dead record inside the first *n* is
        // placed like any other. Each gets "the member's class tile".
        let nondead = self.blackthorn_eligible_party_member_count();
        let class_tile = |party: &[crate::PartyMember], index: usize| {
            party
                .get(index)
                .map(|member| crate::blackthorn::blackthorn_audience_class_tile(member.class_byte))
        };
        for slot in crate::blackthorn::BLACKTHORN_AUDIENCE_FURTHER_PARTY_SLOTS {
            let placed = crate::blackthorn::blackthorn_audience_party_cell(nondead, slot)
                .zip(class_tile(&self.party, slot));
            self.active_objects[slot] = match placed {
                Some(((x, y), tile)) => ActiveObject {
                    type_byte: tile,
                    tile,
                    x,
                    y,
                    z: 0,
                    phase: STEADY_PHASE,
                    aux1: slot as u8,
                    aux3: BLACKTHORN_CUTSCENE_AUX3_ROLE_MARKER,
                },
                None => ActiveObject::empty(),
            };
        }
        for placement in BLACKTHORN_AUDIENCE_ACTOR_PLACEMENTS {
            if !matches!(
                placement.actor,
                BlackthornCutsceneActor::Avatar | BlackthornCutsceneActor::SecondPartyMember
            ) {
                continue;
            }
            if placement.actor == BlackthornCutsceneActor::SecondPartyMember
                && (nondead <= BLACKTHORN_FAILURE_VICTIM_SLOT
                    || self.party.len() <= BLACKTHORN_FAILURE_VICTIM_SLOT)
            {
                continue;
            }
            let slot = placement.actor.slot_index() as usize;
            let actor_byte = class_tile(&self.party, slot).unwrap_or(placement.tile);
            self.place_blackthorn_audience_actor(placement, actor_byte);
        }
        let seated = BlackthornCutsceneActor::SeatedBlackthorn.slot_index() as usize;
        self.active_objects[seated] = ActiveObject::empty();
    }

    /// Timeline step 14: "Place the two guards at (4,10) and (6,10)."
    pub fn install_blackthorn_audience_guards(&mut self) {
        if self.active_objects.len() < OOL_SLOTS {
            self.active_objects.resize(OOL_SLOTS, ActiveObject::empty());
        }
        for placement in BLACKTHORN_AUDIENCE_ACTOR_PLACEMENTS {
            if matches!(
                placement.actor,
                BlackthornCutsceneActor::LeftGuard | BlackthornCutsceneActor::RightGuard
            ) {
                self.place_blackthorn_audience_actor(placement, placement.tile);
            }
        }
        self.mark_visibility_dirty();
    }

    fn place_blackthorn_audience_actor(
        &mut self,
        placement: crate::blackthorn::BlackthornCutsceneActorPlacement,
        actor_byte: u8,
    ) {
        let slot = placement.actor.slot_index() as usize;
        self.active_objects[slot] = ActiveObject {
            type_byte: actor_byte,
            tile: actor_byte,
            x: placement.x,
            y: placement.y,
            z: 0,
            phase: STEADY_PHASE,
            aux1: placement.actor.slot_index(),
            aux3: BLACKTHORN_CUTSCENE_AUX3_ROLE_MARKER,
        };
    }

    pub fn blackthorn_cutscene_vm_from_audience_state(&self) -> BlackthornCutsceneVm {
        let tile_buffer = self
            .blackthorn_audience_map
            .as_ref()
            .map(|map| map.tiles.clone())
            .unwrap_or_else(|| vec![0; MISCMAPS_CUTSCENE_ROWS * MISCMAPS_CUTSCENE_VISIBLE_COLUMNS]);
        let mut vm = BlackthornCutsceneVm::new(tile_buffer);
        for placement in BLACKTHORN_AUDIENCE_ACTOR_PLACEMENTS {
            let slot = placement.actor.slot_index() as usize;
            let Some(object) = self.active_objects.get(slot).copied() else {
                continue;
            };
            if object.is_empty() || object.aux3 != BLACKTHORN_CUTSCENE_AUX3_ROLE_MARKER {
                continue;
            }
            vm.set_actor(
                placement.actor,
                BlackthornCutsceneActorState {
                    x: object.x,
                    y: object.y,
                    actor_byte: object.tile,
                },
            );
        }
        vm.visible_actors = vm.actors;
        vm
    }

    pub fn apply_blackthorn_cutscene_vm_to_audience_state(&mut self, vm: &BlackthornCutsceneVm) {
        if self.active_objects.len() < OOL_SLOTS {
            self.active_objects.resize(OOL_SLOTS, ActiveObject::empty());
        }
        for placement in BLACKTHORN_AUDIENCE_ACTOR_PLACEMENTS {
            let slot = placement.actor.slot_index() as usize;
            if let Some(state) = vm.actor(placement.actor) {
                self.active_objects[slot] = ActiveObject {
                    type_byte: state.actor_byte,
                    tile: state.actor_byte,
                    x: state.x,
                    y: state.y,
                    z: 0,
                    phase: STEADY_PHASE,
                    aux1: placement.actor.slot_index(),
                    aux3: BLACKTHORN_CUTSCENE_AUX3_ROLE_MARKER,
                };
            } else {
                self.active_objects[slot] = ActiveObject::empty();
            }
        }
        match self.blackthorn_audience_map.as_mut() {
            Some(map) => map.tiles = vm.tile_buffer.clone(),
            None => {
                self.blackthorn_audience_map = Some(MiscmapsCutsceneMap {
                    record_index: BLACKTHORN_AUDIENCE_CUTSCENE_MAP_RECORD,
                    tiles: vm.tile_buffer.clone(),
                });
            }
        }
        self.mark_visibility_dirty();
    }

    /// `blackthorn.md §4.2` / §6.3: the conditional seated-Blackthorn tableau
    /// cleanup. "The exit beat's script clears the seated-Blackthorn tableau
    /// slot, and the tableau slot is exactly what the audience tests
    /// afterwards to decide whether to run the conditional Blackthorn-tableau
    /// cleanup vector ... only the punishing branch, which never ran the beat,
    /// reaches it." Returns whether the vector ran.
    pub fn run_blackthorn_conditional_throne_cleanup(&mut self) -> bool {
        let slot = BlackthornCutsceneActor::SeatedBlackthorn.slot_index() as usize;
        let seated = self.active_objects.get(slot).is_some_and(|object| {
            !object.is_empty() && object.aux3 == BLACKTHORN_CUTSCENE_AUX3_ROLE_MARKER
        });
        if seated {
            self.run_blackthorn_cutscene_beat(BlackthornCutsceneBeat::ConditionalThroneCleanup);
        }
        seated
    }

    pub fn run_blackthorn_cutscene_beat(
        &mut self,
        beat: BlackthornCutsceneBeat,
    ) -> BlackthornCutsceneVm {
        let mut vm = self.blackthorn_cutscene_vm_from_audience_state();
        vm.run(blackthorn_cutscene_beat_commands(beat));
        // `audio.md §8.6` / cleak/u5-spec#177: every explicit stinger-pause
        // repetition and every movement step while per-step pauses are enabled
        // runs the live two-part sting. The VM owns the separate two-tick
        // cinematic pause; the speaker program owns only its two rumble halves
        // and calibrated silent gap.
        for _ in 0..vm.stinger_count {
            self.emit_sound_effect(SoundEffect::BlackthornMovementStinger);
        }
        self.apply_blackthorn_cutscene_vm_to_audience_state(&vm);
        vm
    }

    pub fn submit_blackthorn_audience_answer(
        &mut self,
        typed: &str,
        game_dir: &Path,
    ) -> io::Result<MoveOutcome> {
        // `blackthorn.md §3`: the audience's first key read is the greeting
        // acknowledgement, after the whole capture narration (issue #304
        // Q6, Q7). A key that reaches here while the narration is still
        // staged - a shell with no pump - finishes it first, so the greeting
        // is on screen before its acknowledgement runs.
        if self.blackthorn_capture_narration_active() {
            self.flush_staged_narration();
        }
        // `blackthorn.md §5` step 5: "Wait for player acknowledgement before
        // returning to the caller branch." The outcome is on screen and the
        // durable consequences have run; this key is what returns the party
        // to the captive cell.
        if self.active_blackthorn.as_ref().is_some_and(
            crate::blackthorn_session::BlackthornChallenge::awaiting_closing_acknowledgement,
        ) {
            let challenge = self.active_blackthorn.take().expect("checked above");
            // `blackthorn.md §5` (u5-spec#298 Q31): the blade hold "is not a
            // key wait"; the execution lands at its end, before this key's
            // wait begins. A key that reaches here while the hold is still
            // queued - a shell with no pump - finishes it first, so the roster
            // edit still precedes whatever the key does.
            if self.blackthorn_execution_hold_pending() {
                self.flush_staged_narration();
            }
            if challenge.closing_epilogue_pending() {
                // `blackthorn.md §5`: "After acknowledgement, record `6`
                // supplies the quoted unfairness/treachery speech, including
                // its leading two line feeds. The execution helper finishes
                // with one line feed."
                // That line feed ends the punishment routine, and the loop
                // leaves through its own tail with no further key wait.
                let speech = self.blackthorn_reaction_text(
                    game_dir,
                    crate::MISCMSG_BLACKTHORN_TREACHERY_SPEECH,
                    "",
                )?;
                let _ = challenge;
                self.emit_message_line(speech);
                self.push_explicit_blank_message_entry();
                self.blackthorn_audience_punished = false;
                self.run_blackthorn_conditional_throne_cleanup();
                return self.apply_blackthorn_captive_cell_handoff(game_dir, "");
            }
            // `blackthorn.md §4.2`, the audience exit beat. This key is the
            // acknowledgement *and* the beat's own trigger
            // (`cleak/u5-spec#268`: "One, and it *is* the acknowledgement.
            // The record is printed and the very next instruction is the
            // beat's key wait"), and the animation after it "consumes no
            // input ... so it cannot be skipped". The window is untouched
            // across it, so the hold prints nothing.
            if std::mem::take(&mut self.blackthorn_audience_punished) {
                // The punishing branches skip the beat; they alone reach the
                // conditional tableau cleanup (§4.2).
                self.run_blackthorn_conditional_throne_cleanup();
                return self.apply_blackthorn_captive_cell_handoff(game_dir, "");
            }
            // §4.2: "After the key, the beat hands the exit vector to the
            // cutscene script VM" - §6.3's audience-exit row, whose stingers
            // are emitted and whose final frame clears the tableau slot. The
            // hold is §4.2's re-measured "about 73 tick periods" to the end of
            // the script, then the handoff and cell-entry setup's untimed
            // "about 0.27 s (about 5 tick periods)" (u5-spec#301 Q4, R613).
            let vm = self.run_blackthorn_cutscene_beat(BlackthornCutsceneBeat::AudienceExit);
            self.push_diagnostic(format!(
                "Blackthorn audience exit beat: {} stings, {} world ticks.",
                vm.stinger_count, vm.world_ticks
            ));
            self.staged_narration.push_ticks(
                crate::blackthorn::BLACKTHORN_AUDIENCE_EXIT_BEAT_BIOS_TICKS
                    + crate::blackthorn::BLACKTHORN_CAPTIVE_CELL_HANDOFF_BIOS_TICKS,
                String::new(),
            );
            self.pending_blackthorn_audience_exit = true;
            return Ok(MoveOutcome::Observed);
        }

        // `blackthorn.md §4.1`: the wrong-answer reaction is followed by
        // "acknowledgement, then `\n\n` before the second ask", so while the
        // loop is holding on that reaction any key advances it and nothing is
        // read as an answer.
        if self
            .active_blackthorn
            .as_ref()
            .is_some_and(crate::blackthorn_session::BlackthornChallenge::awaiting_acknowledgement)
        {
            let mut challenge = self.active_blackthorn.take().expect("checked above");
            let outcome = challenge.submit("");
            let message = match outcome {
                // `blackthorn.md §3` steps 8-9: the greeting's key releases
                // the guard-release route, then record `11` is printed and
                // held for the audience's second acknowledgement.
                crate::blackthorn_session::BlackthornChallengeOutcome::GreetingAcknowledged => {
                    // §3 step 8 (u5-spec#298 Q8): the release order, chosen by
                    // roster record 0's gender byte, printed before the
                    // guard-release route and ending mid-row ("no trailing
                    // space, no line feed").
                    let release_line = crate::blackthorn::blackthorn_release_line(
                        self.blackthorn_roster_record_zero_gender(),
                    );
                    self.emit_message_line(release_line.clone());
                    self.open_message_row();
                    // "Then run the guard-release route script of Section
                    // 6.3", and step 9: "Print the preamble record". The
                    // route takes its "quiet pause 11" and six steps "with a
                    // stinger-plus-two-tick redraw after every step" before
                    // the preamble, so both are staged; it used to run in one
                    // go and print the preamble with the release order
                    // (`bt-capture-pacing`/`s06`, `bt-correct`/`typed`,
                    // 2026-10-27: the stock game still shows the order and
                    // the guard walking).
                    let preamble = self.blackthorn_audience_opening_text(game_dir)?;
                    self.stage_blackthorn_guard_release_route(&release_line, preamble);
                    self.active_blackthorn = Some(challenge);
                    return Ok(MoveOutcome::PromptDeclined);
                }
                crate::blackthorn_session::BlackthornChallengeOutcome::PromptPresented {
                    ordinal,
                    prompt,
                } => {
                    let demand = self.blackthorn_demand_text(game_dir, ordinal, prompt)?;
                    if ordinal == 0 {
                        // `blackthorn.md §4.1`: record `11`, "then
                        // acknowledgement and two line feeds before the first
                        // demand". The preamble leaves its row open, so the
                        // first feed closes it and the second is the blank row
                        // the stock game shows above `"What is the Mantra`.
                        self.flush_message_slot();
                        self.open_message_row();
                        format!("\n\n{demand}\n\n{BLACKTHORN_INPUT_PROMPT}")
                    } else {
                        // §4.1, first wrong answer: the threat, then
                        // "acknowledgement, then `\n\n` before the second
                        // ask" (§5 likewise). The threat ends in an open row
                        // after `die!" `, so the first feed closes it and the
                        // second leaves the blank row above `"Now tell me,`
                        // - the same shape as the preamble above (playtest
                        // H40).
                        self.flush_message_slot();
                        self.open_message_row();
                        format!("\n\n{demand}\n\n{BLACKTHORN_INPUT_PROMPT}")
                    }
                }
                _ => String::new(),
            };
            self.active_blackthorn = Some(challenge);
            self.message = message;
            return Ok(MoveOutcome::PromptDeclined);
        }

        let answer = blackthorn_challenge_limited_input(typed);
        // `input.md §8`: "Enter then submits whatever remains, including an
        // empty buffer", so an empty answer at the demand is submitted and
        // fails the mantra match like any other wrong answer. Only a session
        // that is not asking anything re-renders its page (playtest H39).
        if answer.is_empty()
            && !self
                .active_blackthorn
                .as_ref()
                .is_some_and(|challenge| challenge.current_prompt().is_some())
        {
            self.message = self.blackthorn_current_prompt_message(game_dir)?;
            return Ok(MoveOutcome::PromptDeclined);
        }

        let Some(mut challenge) = self.active_blackthorn.take() else {
            self.message.clear();
            return Ok(MoveOutcome::Blocked);
        };

        // `blackthorn.md §4.1`: "The bounded text input then accepts the
        // answer; submission is followed by `\n\n` before the outcome." The
        // answered row is part of the transcript, so it is committed here -
        // `qa/tools/paired_compare.py` read `bt-correct`'s `right` beat with
        // the stock game still showing `:AHM` above the reward speech and
        // this engine having dropped it.
        // The answer row is a *live* row, not an open transcript entry, so
        // `commit_prompt_reply` has nothing to append the typed word to; the
        // finished row is emitted as its own line instead.
        self.emit_message_line(format!(
            "{BLACKTHORN_ANSWER_ROW_PREFIX}{}",
            answer.to_ascii_uppercase()
        ));
        self.push_explicit_blank_message_entry();

        // `blackthorn.md §4`: "The shrine index is fixed before the loop
        // starts and never changes inside it", so the consequence arms read
        // the session's index rather than re-scanning the ruin flags — a
        // re-scan would drift onto the next shrine the moment this one is
        // ruined.
        let shrine_index = usize::from(challenge.shrine_index);

        match challenge.submit(&answer) {
            // `BlackthornChallenge::submit` reports a matching answer as
            // `Survived`; `Correct` is never produced, so it shares the arm
            // rather than carrying a second, divergent copy of the outcome.
            crate::blackthorn_session::BlackthornChallengeOutcome::Correct { .. }
            | crate::blackthorn_session::BlackthornChallengeOutcome::Survived => {
                // `blackthorn.md §4`: a correct answer resolves the
                // interrogation, and it is the answer — not surviving the
                // ladder — that ruins the shrine and debits five points of
                // moral standing.
                self.apply_blackthorn_correct_answer_consequences(shrine_index);
                let standing = self.moral_standing;
                // §4 counts "all nondead travelling party members at audience
                // setup", which is the count the fate branch itself uses, so
                // it is read before the fate resolves.
                let nondead = self.blackthorn_eligible_party_member_count();
                let fate = self.blackthorn_correct_answer_companion_fate_preview();
                // `blackthorn.md §4.2`: the conditional tableau cleanup is
                // reached "only [by] the punishing branch"; this branch plays
                // the exit beat, which clears the tableau slot the cleanup
                // tests, so it skips it.
                self.push_diagnostic(format!(
                    "Survived Blackthorn's challenge; shrine {} is ruined; standing {standing}; {fate}.",
                    shrine_index + 1,
                ));
                // `blackthorn.md §4.1`: "Correct answer with at least two
                // nondead members | Record `5`, the merciful-death speech";
                // "Correct answer with only one nondead member | Record `9`,
                // the truth/life reward speech". The engine printed the
                // sentence above instead - `qa/tools/paired_compare.py` read
                // `Survived Blackthorn's challenge; ... advanced 17 world
                // ticks without clearing the screen. Returned to
                // Blackthorn's captive cell ...` on `bt-correct`'s `right`
                // beat where the stock game printed the speech.
                let reward = if nondead >= 2 {
                    self.blackthorn_reaction_text(
                        game_dir,
                        crate::MISCMSG_BLACKTHORN_MERCIFUL_DEATH,
                        BLACKTHORN_MERCIFUL_DEATH_LINE,
                    )?
                } else {
                    self.blackthorn_reaction_text(
                        game_dir,
                        crate::MISCMSG_BLACKTHORN_TRUTH_REWARD,
                        "",
                    )?
                };
                let executes = nondead >= 2;
                // `blackthorn.md §5` (u5-spec#298 Q24, Q31): record `5`, then
                // the execution's quiet pause, swell and impact - "not a key
                // wait" - with the companion leaving the panel "at the end of
                // the blade-and-impact hold", before the exit beat's key wait
                // begins. The engine previously deferred the lift to that key
                // after a stock capture taken with a key sent during the hold.
                let outcome = self.hold_blackthorn_closing(challenge, reward.clone())?;
                if executes {
                    // §5 "Rows on the correct-answer arm" (u5-spec#299 Q9):
                    // record `5` starts at column 0 and its last wrapped row,
                    // `merciful death!"`, "fills all sixteen cells, so the
                    // printer's own wrap leaves the cursor at column 0 of the
                    // next row as the record finishes". "That empty row under
                    // the record is the advance a capture sees through the
                    // whole blade hold" (`bt-correct`'s `right` beat). It is
                    // the record's own wrap, not step 9's feed, which still
                    // follows the roster edit
                    // ([`Self::release_blackthorn_execution_hold`]).
                    let fills_last_row = crate::wrap_text(
                        &reward,
                        crate::message_window::MESSAGE_WINDOW_WIDTH,
                        0,
                    )
                    .last()
                    .is_some_and(|row| {
                        row.chars().count() == crate::message_window::MESSAGE_WINDOW_WIDTH
                    });
                    if fills_last_row {
                        self.push_explicit_blank_message_entry();
                    }
                    self.pending_blackthorn_merciful_death = true;
                    self.stage_blackthorn_execution_hold(None);
                } else {
                    self.leave_solo_closing_record_row_open();
                }
                Ok(outcome)
            }
            crate::blackthorn_session::BlackthornChallengeOutcome::Wrong { ordinal, expected } => {
                // `blackthorn.md §4`: "**A wrong answer, when few companions
                // remain, ends the interrogation** with a mocking line about
                // lying and a threat of the dungeon. **A wrong answer
                // otherwise escalates.** The first wrong answer produces a
                // threat naming the companion at risk. Later wrong answers
                // stamp a tile into the cutscene map, and the fourth wrong
                // answer **kills** the named companion with the pendulum-blade
                // narration."
                //
                // §5 scopes the punishing side to "a branch that can punish a
                // companion" and names the victim as "the second living party
                // member (the first living companion behind the Avatar)", so
                // the ending branch is the one where no such companion exists.
                // §4 does not quantify "few" beyond that, so nothing narrower
                // is assumed here.
                // `blackthorn.md §4`: "A wrong answer, when fewer than two
                // nondead party members remain, ends the interrogation".
                let victim = self
                    .blackthorn_failure_victim_index()
                    .filter(|index| {
                        *index != 0 && self.blackthorn_eligible_party_member_count() >= 2
                    });
                let Some(victim) = victim else {
                    // The ordinal and the expected mantra are harness detail,
                    // not player text.
                    self.push_diagnostic(format!(
                        "Failed Blackthorn's prompt {}; expected {expected}.",
                        ordinal + 1
                    ));
                    let line = self.blackthorn_reaction_text(
                        game_dir,
                        crate::MISCMSG_BLACKTHORN_DUNGEON_THREAT,
                        BLACKTHORN_DUNGEON_THREAT,
                    )?;
                    let outcome = self.hold_blackthorn_closing(challenge, line)?;
                    self.leave_solo_closing_record_row_open();
                    return Ok(outcome);
                };
                match challenge.wrong_escalation() {
                    // First wrong answer: a threat only. No tile is stamped
                    // yet, and the loop re-asks with the next wording.
                    Some(crate::blackthorn_session::BlackthornWrongEscalation::Threat) => {
                        let (next_ordinal, prompt) = challenge
                            .current_prompt()
                            .unwrap_or((ordinal + 1, "Virtue"));
                        // `blackthorn.md §4` / §4.1 (R441): the threat names
                        // "the second roster entry", fixed, whatever its
                        // status; the execution victim is separately the
                        // second nondead member, so the two can differ.
                        let victim_name = self.blackthorn_victim_display_name(
                            crate::blackthorn::BLACKTHORN_FAILURE_VICTIM_SLOT,
                        );
                        let _ = prompt;
                        challenge.await_acknowledgement(next_ordinal);
                        self.active_blackthorn = Some(challenge);
                        self.push_diagnostic(format!(
                            "Failed Blackthorn's prompt {}; expected {expected}; victim slot {}.",
                            ordinal + 1,
                            victim + 1,
                        ));
                        // `blackthorn.md §4.1`, the first wrong answer with at
                        // least two nondead members: record `7`, then record
                        // `8` with the second roster entry's name, then
                        // "acknowledgement, then `\n\n` before the second
                        // ask". The engine stopped after the threat, so the
                        // second demand - §4.1's record `1` - never printed
                        // and the player answered a question that was not on
                        // screen.
                        // `blackthorn.md §4.1`: record `7`, "the quoted
                        // laughing-at-me rebuke; after its scene beat, record
                        // `8`, which begins with two line feeds and the
                        // quoted sand/threat prefix; append the second roster
                        // entry's name and ` die!\" `". Record `8` carries
                        // its own leading line feeds and its trailing space,
                        // so the composition adds neither - the cursor sits
                        // one cell further right than the engine had it on
                        // `bt-escalate`'s `ask2` beat.
                        let rebuke = self.blackthorn_reaction_text(
                            game_dir,
                            crate::MISCMSG_BLACKTHORN_FIRST_WRONG_REBUKE,
                            BLACKTHORN_FIRST_WRONG_LINE,
                        )?;
                        // §4.1: record `8` comes "after its scene beat", and
                        // §6.1: "The script swap listed in Section 6.3 (`0x80`
                        // becomes `0x82`, `0xE8` becomes `0xE9`) runs on the
                        // **first** wrong answer only" - the failed-challenge
                        // vector, which §4.2's table names as this branch's
                        // failure reaction.
                        self.run_blackthorn_cutscene_beat(
                            BlackthornCutsceneBeat::FailedChallengeReaction,
                        );
                        let threat = self.blackthorn_reaction_text(
                            game_dir,
                            crate::MISCMSG_BLACKTHORN_SAND_THREAT_PREFIX,
                            BLACKTHORN_SAND_THREAT_PREFIX_FALLBACK,
                        )?;
                        self.message = format!("{rebuke}{threat}{victim_name} die!\" ");
                        Ok(MoveOutcome::PromptDeclined)
                    }
                    // Second and third wrong answers: "Later wrong answers
                    // stamp a tile into the cutscene map". They do not run
                    // §5's execution beat or clear the victim actor; that
                    // happens only on the fourth answer.
                    //
                    // `blackthorn.md §6.1`: "an open-coded stamp selected by
                    // question index writes the hourglass cell directly ...
                    // index 1 writes `0xEB`, index 2 writes `0xE8`". The
                    // pendulum is not stamped here: the first wrong answer's
                    // script swap already placed it. §4.2's table (u5-spec#298
                    // Q8): "A two-minute per-turn cleanup ..., the stats
                    // repaint and the hourglass stamp of Section 6.1, then the
                    // loop asks again".
                    Some(crate::blackthorn_session::BlackthornWrongEscalation::TileStamp) => {
                        self.run_blackthorn_wrong_answer_cleanup();
                        let (x, y) = crate::blackthorn::BLACKTHORN_HOURGLASS_CELL;
                        let mut commands = Vec::with_capacity(2);
                        if let Some(tile) = crate::blackthorn::blackthorn_hourglass_stamp(ordinal) {
                            commands.push(BlackthornCutsceneCommand::WriteTerrain { x, y, tile });
                        }
                        commands.push(BlackthornCutsceneCommand::ExplicitRedraw);
                        let mut vm = self.blackthorn_cutscene_vm_from_audience_state();
                        vm.run(&commands);
                        self.apply_blackthorn_cutscene_vm_to_audience_state(&vm);
                        let (next_ordinal, prompt) = challenge
                            .current_prompt()
                            .unwrap_or((ordinal + 1, "Virtue"));
                        let demand = self.blackthorn_demand_text(game_dir, next_ordinal, prompt)?;
                        self.active_blackthorn = Some(challenge);
                        self.push_diagnostic(format!(
                            "Failed Blackthorn's prompt {}; expected {expected}; victim slot {}; cutscene advanced {} world tick.",
                            ordinal + 1,
                            victim + 1,
                            vm.world_ticks,
                        ));
                        // The third ask re-states the question and the fourth
                        // shouts it; both come from §4.1's own records rather
                        // than from the ordinal the engine happened to be on.
                        self.message = format!("{demand}\n\n{BLACKTHORN_INPUT_PROMPT}");
                        Ok(MoveOutcome::PromptDeclined)
                    }
                    // Fourth wrong answer: the §5 execution. `blackthorn.md
                    // §6.1`: "index 3 executes a companion (Section 5) rather
                    // than stamping", and the §6.3 swap script ran on the
                    // first wrong answer only, so it is not replayed here.
                    _ => {
                        // §4.2 (u5-spec#298 Q8): "The same two-minute cleanup
                        // and stats repaint, then the pendulum execution".
                        self.run_blackthorn_wrong_answer_cleanup();
                        // §5: the tail names "the actual removed member",
                        // read before the roster record is lifted out.
                        let victim_name = self.blackthorn_victim_display_name(victim);
                        // `blackthorn.md §5` (u5-spec#298 Q24, R532): the
                        // execution "runs no cutscene script and moves no
                        // guard" - the failed-challenge script and its swap
                        // ran on the first wrong answer only (§6.1), so the
                        // hourglass stays on the `0xE8` the third answer left.
                        //
                        // Q31: record `4` is not a key wait. "The panel lists
                        // the whole party through the pendulum-narration page
                        // and the blade, and the reduced party first appears
                        // in the same screenshot sample as the `... is sliced
                        // in half!` page, which then waits for the key." The
                        // engine had held the narration for a key and lifted
                        // the victim on it, after a stock capture with a key
                        // sent during the blade hold.
                        self.pending_blackthorn_execution_victim = Some(victim);
                        // This is the punishing branch, so no §4.2 exit beat
                        // follows its acknowledgement (`cleak/u5-spec#268`).
                        self.blackthorn_audience_punished = true;
                        self.push_diagnostic(format!(
                            "Failed Blackthorn's prompt {}; expected {expected}; companion in slot {victim} is owed the pendulum blade.", // audit: not a player-facing line
                            ordinal + 1,
                        ));
                        let line = self.blackthorn_reaction_text(
                            game_dir,
                            crate::MISCMSG_BLACKTHORN_PENDULUM_NARRATION,
                            BLACKTHORN_PENDULUM_NARRATION,
                        )?;
                        // §5 step 1 prints record `4` as it reads, with no
                        // line feed: the row stays open after `falls!`, and
                        // step 8's leading `\n\n` closes it and spends the one
                        // blank row above the sliced line. The feed this
                        // engine used to append here (from the stock `ask5`
                        // beat, which showed `falls!` with no cursor and no
                        // row under it) was the helper's closing feed of step
                        // 9 read onto the narration; that sample was taken
                        // inside the blade hold, which draws no cursor
                        // because it is not a key wait (Q31) - as this
                        // engine's staged hold does not either. With the
                        // feed, the sliced line sat under two blank rows.
                        // The key wait that follows belongs to the sliced
                        // page (§5 step 8), so the session waits for its
                        // closing key from here, with record `6` behind it.
                        let outcome =
                            self.hold_blackthorn_closing_page(challenge, line, true)?;
                        self.open_message_row();
                        // §5 step 8: "On the wrong-answer arm only: print
                        // `\n\n`, the removed member's name and ` is sliced
                        // in half! `, wait for an acknowledgement key" - after
                        // the roster change of step 7, so it is staged behind
                        // the hold.
                        self.stage_blackthorn_execution_hold(Some(format!(
                            "\n\n{victim_name} is sliced in half! "
                        )));
                        Ok(outcome)
                    }
                }
            }
            crate::blackthorn_session::BlackthornChallengeOutcome::PromptPresented {
                ordinal,
                prompt,
            } => {
                let demand = self.blackthorn_demand_text(game_dir, ordinal, prompt)?;
                self.active_blackthorn = Some(challenge);
                // The same §4.1 demand the session renders elsewhere; this arm
                // used to print `Blackthorn asks for <virtue>.`
                self.message = format!("{demand}\n\n{BLACKTHORN_INPUT_PROMPT}");
                Ok(MoveOutcome::PromptDeclined)
            }
            crate::blackthorn_session::BlackthornChallengeOutcome::AlreadyPunished => self
                .blackthorn_resolved_handoff(
                    game_dir,
                    "Blackthorn's punishment has already resolved.",
                ),
            crate::blackthorn_session::BlackthornChallengeOutcome::AlreadySurvived => self
                .blackthorn_resolved_handoff(
                    game_dir,
                    "Blackthorn's challenge has already resolved.",
                ),
            crate::blackthorn_session::BlackthornChallengeOutcome::AlreadyAborted => {
                self.blackthorn_resolved_handoff(game_dir, "Blackthorn's challenge was aborted.")
            }
            // Setup acknowledgements are consumed above, before any answer is
            // read; reaching here only re-renders the held page.
            crate::blackthorn_session::BlackthornChallengeOutcome::GreetingAcknowledged => {
                self.active_blackthorn = Some(challenge);
                Ok(MoveOutcome::PromptDeclined)
            }
        }
    }

    /// Whether a `blackthorn.md §5` execution is staged and its roster edit
    /// has not landed yet.
    pub fn blackthorn_execution_hold_pending(&self) -> bool {
        self.pending_blackthorn_merciful_death || self.pending_blackthorn_execution_victim.is_some()
    }

    /// `blackthorn.md §5` steps 2-4 (u5-spec#298 Q24): after the record, "A
    /// quiet 10-tick redraw pause", "The 920-run swell, the Blackthorn
    /// execution row of `systems/audio.md` Section 5.4.7", then "The shared
    /// impact presentation ... the impact figure is drawn, the 300-update
    /// impact rumble plays, and one world tick follows". No inversion and no
    /// guard movement. Two staged pure holds with nothing to print: the
    /// quiet pause, at whose end the swell and the impact rumble sound
    /// ("Nothing else happens between the record and the swell"), then the
    /// sounds' own length, at whose end the roster edit of steps 5-7 lands
    /// ([`Self::release_blackthorn_execution_hold`]). `page` - the wrong
    /// arm's sliced line - prints after that, continuing the row record `4`
    /// left open.
    fn stage_blackthorn_execution_hold(&mut self, page: Option<String>) {
        self.pending_blackthorn_execution_swell = true;
        self.staged_narration.push_ticks(
            crate::blackthorn::BLACKTHORN_EXECUTION_QUIET_PAUSE_TICKS,
            String::new(),
        );
        let mut jitter = crate::audio::RumbleJitter::default();
        let sounds = SoundEffect::BlackthornExecutionSwell
            .program(&mut jitter)
            .duration(true)
            .as_secs_f64()
            + SoundEffect::RoughSeasImpactRumble
                .program(&mut jitter)
                .duration(true)
                .as_secs_f64();
        self.staged_narration.push_secs(sounds, String::new());
        if let Some(page) = page {
            self.staged_narration.push_secs_on_open_row(0.0, page);
        }
    }

    /// `blackthorn.md §5` steps 4-7, at the end of the blade hold: the impact
    /// lands at "the cell where the second-member actor (slot 1) currently
    /// stands", that actor is cleared, "the pendulum cell `(5,7)`" is reset
    /// "to the fixture's unoccupied member `0x80`", and the second nondead
    /// member is lifted with the stats panel "repaint[ed] **immediately
    /// afterwards**". Called as each staged beat releases; a no-op unless an
    /// execution is pending. The first release after staging is the end of
    /// the quiet pause, which starts the swell and the rumble (steps 3-4);
    /// the next is the end of those sounds, which runs the roster edit.
    pub(crate) fn release_blackthorn_execution_hold(&mut self) {
        if std::mem::take(&mut self.pending_blackthorn_execution_swell) {
            self.emit_sound_effect(SoundEffect::BlackthornExecutionSwell);
            self.emit_sound_effect(SoundEffect::RoughSeasImpactRumble);
            return;
        }
        let merciful = std::mem::take(&mut self.pending_blackthorn_merciful_death);
        let victim = self.pending_blackthorn_execution_victim.take();
        if !merciful && victim.is_none() {
            return;
        }
        // Step 4's cell. After a first wrong answer the actor sprite was
        // cleared on the pendulum, where §5 puts it; this engine's script VM
        // drops a cleared actor's coordinates, so that case reads the
        // published pendulum cell.
        let slot = BlackthornCutsceneActor::SecondPartyMember.slot_index() as usize;
        let impact = self
            .active_objects
            .get(slot)
            .filter(|object| !object.is_empty())
            .map_or(crate::blackthorn::BLACKTHORN_PENDULUM_CELL, |object| (object.x, object.y));
        // Step 6: buffered, shown on the next redraw.
        let (x, y) = crate::blackthorn::BLACKTHORN_PENDULUM_CELL;
        if let Some(index) = crate::blackthorn::blackthorn_cutscene_tile_index(x, y) {
            if let Some(cell) = self
                .blackthorn_audience_map
                .as_mut()
                .and_then(|map| map.tiles.get_mut(index))
            {
                *cell = crate::blackthorn::BLACKTHORN_PENDULUM_UNOCCUPIED_TILE;
            }
        }
        // Steps 5 and 7 (the actor clear is part of the shared routine).
        let report = if merciful {
            self.apply_blackthorn_correct_answer_companion_fate()
        } else {
            victim
                .and_then(|victim| self.execute_blackthorn_companion(victim))
                .unwrap_or_else(|| "no companion remains to punish".to_string())
        };
        self.repaint_stats_panel();
        // §5 step 9 on the correct-answer arm (u5-spec#299 Q9): "Step 9's
        // line feed is a separate emit, so it always costs one more row. It
        // appears together with the reduced panel and leaves one blank row
        // between the record's last row and the exit beat's key-wait cursor
        // ... Do not move it ahead of the roster edit." After a row-filling
        // record the cursor is already at column 0, so the feed leaves a
        // blank row; after one that stopped mid-row it only closes that row.
        // The wrong-answer arm's feed follows record `6`.
        if merciful {
            if self.message_row_open_mid_line {
                self.close_message_row();
            } else {
                self.push_explicit_blank_message_entry();
            }
        }
        self.push_diagnostic(format!(
            "Blackthorn execution at the end of the blade hold (impact at {impact:?}): {report}."
        ));
    }

    /// `blackthorn.md §5` (u5-spec#298 Q8): "Wrong answers past the first
    /// advance the clock by two minutes (the per-turn cleanup's two-minute
    /// increment of `systems/time.md` Section 3; Quickness halves it and
    /// Negate Time suppresses it, Section 4) and then repaint unconditionally
    /// with the roster intact; the first wrong answer makes no cleanup call".
    fn run_blackthorn_wrong_answer_cleanup(&mut self) {
        let effective_minutes = if self.negate_time_active() {
            0
        } else {
            TimingStatusTag::from_save_byte(self.active_effect_tag.unwrap_or(0))
                .effective_minutes(crate::blackthorn::BLACKTHORN_WRONG_ANSWER_CLEANUP_MINUTES)
        };
        self.advance_clock_half(effective_minutes);
        self.repaint_stats_panel();
    }

    /// `blackthorn.md §3` step 8: "the name in **roster record 0** (the
    /// Avatar's record ...)". The saved roster's first record when one is
    /// loaded, else the leader's panel name.
    fn blackthorn_roster_record_zero_name(&self) -> String {
        let raw = self
            .party_roster
            .first()
            .map(|record| record.name)
            .or_else(|| self.party_names.first().copied());
        raw.map(|name| {
            String::from_utf8_lossy(&name)
                .trim_end_matches('\0')
                .trim()
                .to_string()
        })
        .unwrap_or_default()
    }

    /// `blackthorn.md §3` step 8: "record 0's gender byte
    /// (`formats/saved-gam.md`)". With no roster loaded there is no byte to
    /// read, which takes the spec's "nothing for any other value" arm.
    fn blackthorn_roster_record_zero_gender(&self) -> u8 {
        self.party_roster.first().map_or(0, |record| record.gender)
    }

    /// The name Blackthorn's threat uses for the companion at risk, read
    /// from the roster the panel draws from.
    fn blackthorn_victim_display_name(&self, victim: usize) -> String {
        self.party_names
            .get(victim)
            .map(|name| {
                String::from_utf8_lossy(name)
                    .trim_end_matches('\0')
                    .trim()
                    .to_string()
            })
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| "thy companion".to_string())
    }

    pub fn blackthorn_current_prompt_message(&self, game_dir: &Path) -> io::Result<String> {
        let Some(challenge) = self.active_blackthorn.as_ref() else {
            // The session cannot be absent when this renders; the string is a
            // harness fallback, never a printed line.
            return Ok("Blackthorn audience is not active.".to_string()); // audit: not a player-facing line
        };
        if let Some((ordinal, prompt)) = challenge.current_prompt() {
            // `blackthorn.md §4` withdrawal: "the loop's party-slot
            // argument is semantic: it names which companion is at risk"
            // is withdrawn. "The loop asks about ONE shrine, up to four
            // times", so the prompt names only that shrine's virtue.
            // **Measured** 2026-09-08 in Blackthorn's palace
            // (`qa/paired/bt-audience.tsv`): the demand is quoted, names the
            // shrine as "the Mystic Shrine of <virtue>", and ends on its own
            // typed-input prompt.
            //
            // ```text
            // "What is the
            // Mantra of the
            // Mystic Shrine of
            // Honesty?"
            //
            // Your response?
            // ```
            //
            // The engine printed `Blackthorn asks for the mantra of Honesty.`
            let demand = self.blackthorn_demand_text(game_dir, ordinal, prompt)?;
            Ok(format!("{demand}\n\n{BLACKTHORN_INPUT_PROMPT}"))
        } else {
            Ok("Blackthorn waits.".to_string())
        }
    }

    /// `blackthorn.md §3` step 2: "scan the eight shrine ruin flags in
    /// shrine order and take the first whose flag is *exactly* clear — never
    /// ruined and never restored."
    ///
    /// "Exactly clear" is a whole-byte test, not a high-bit test: the
    /// restoration path at the Word-of-Power arm only clears the ruin bit, so
    /// a restored shrine keeps a non-zero flag byte and is skipped here.
    pub fn blackthorn_selected_shrine(&self) -> Option<usize> {
        (0..SAVE_SHRINE_RUIN_FLAG_COUNT).find(|index| self.shrine_ruin_flags[*index] == 0)
    }

    /// `blackthorn.md §4`: "A correct answer ruins that shrine and costs five
    /// points of moral standing. The shrine's durable ruin flag is set, so
    /// the shrine thereafter renders and behaves as a ruined shrine until the
    /// player restores it by meditating there. The moral-standing debit is a
    /// clamped subtraction of five, floored at zero."
    ///
    /// The §4 withdrawal reverses the earlier "the challenge does not
    /// directly adjust numeric karma" reading: the interrogation *does* debit
    /// moral standing. The ruin bit written here is the same
    /// `SAVE_QUEST_TILE_FLAG_HIGH_BIT` the overworld quest-tile pass reads to
    /// render a ruined shrine and the shrine-restoration arm clears.
    pub fn apply_blackthorn_correct_answer_consequences(&mut self, shrine_index: usize) -> bool {
        let Some(flag) = self.shrine_ruin_flags.get_mut(shrine_index) else {
            return false;
        };
        *flag |= SAVE_QUEST_TILE_FLAG_HIGH_BIT;
        self.moral_standing = self
            .moral_standing
            .saturating_sub(Self::BLACKTHORN_CORRECT_ANSWER_STANDING_DEBIT);
        true
    }

    /// `blackthorn.md §3`: the audience setup "counts the active party
    /// members that are still eligible for the challenge". The withdrawn
    /// reading made this an eight-slot scan for "the first party slot
    /// ... whose per-member Blackthorn-jail flag is still clear";
    /// §3 answers that in full: "**There is no per-member jail flag.**"
    /// Eligibility is liveness.
    pub fn blackthorn_eligible_party_member_count(&self) -> usize {
        self.party.iter().filter(|member| member.living()).count()
    }

    /// `blackthorn.md §5`: "The victim is the second living party member
    /// (the first living companion behind the Avatar)". Nothing is
    /// flagged on the victim's record - the earlier
    /// `mark_blackthorn_failure_victim_handled` wrote a durable jail bit,
    /// which §8 withdraws.
    pub fn blackthorn_failure_victim_index(&self) -> Option<usize> {
        self.party
            .iter()
            .enumerate()
            .filter(|(_, member)| member.living())
            .map(|(index, _)| index)
            .nth(BLACKTHORN_FAILURE_VICTIM_SLOT)
            .or_else(|| self.party.iter().position(|member| member.living()))
    }

    /// `blackthorn.md §4`: "**A correct answer also decides a
    /// companion's fate.** The threshold counts all nondead travelling party
    /// members at audience setup, including Avatar when nondead. With at
    /// least two, Blackthorn thanks the player and **kills** the second
    /// nondead member as 'a merciful death'. With only one, he spares that
    /// survivor instead." (R440 withdrew the more-than-one-companion rule.)
    ///
    /// §5 gives the execution's shape: the routine "erases that
    /// companion's on-screen actor; lifts their roster record out of the
    /// party, compacts the remaining records up, and decrements the
    /// party count; parks the lifted record in the last roster slot with
    /// a **whereabouts value that matches no location**." §8 lists the
    /// result as "Durable and irreversible". The withdrawn reading
    /// substituted a per-member jail flag for the death, so the roster
    /// never shrank.
    /// What [`Self::apply_blackthorn_correct_answer_companion_fate`] will do
    /// when the acknowledgement lands, without doing it.
    ///
    /// The submit arm records the outcome for the harness while the reaction
    /// is still on screen; the roster itself is not touched until the player
    /// acknowledges (`blackthorn.md §5` step 5).
    pub fn blackthorn_correct_answer_companion_fate_preview(&self) -> String {
        // `blackthorn.md §4` (R440): the threshold "counts all nondead
        // travelling party members ..., including Avatar when nondead. With at
        // least two, Blackthorn ... **kills** the second nondead member".
        if self.blackthorn_eligible_party_member_count() < 2 {
            return "only one companion remains, so Blackthorn spares the player".to_string(); // audit: not a player-facing line
        }
        match self.blackthorn_failure_victim_index() {
            Some(0) | None => {
                "no companion remains to take the merciful death".to_string() // audit: not a player-facing line
            }
            Some(index) => format!("companion in slot {index} is owed a merciful death"), // audit: not a player-facing line
        }
    }

    pub fn apply_blackthorn_correct_answer_companion_fate(&mut self) -> String {
        // `blackthorn.md §4` (R440): Avatar plus one living companion is
        // enough for the execution branch; the earlier more-than-one-companion
        // requirement is withdrawn.
        if self.blackthorn_eligible_party_member_count() < 2 {
            // audit: not a player-facing line - the fate string is a harness
            // diagnostic, and the measured scene says only `merciful death!`.
            return "only one companion remains, so Blackthorn spares the player".to_string(); // audit: not a player-facing line
        }
        let Some(index) = self.blackthorn_failure_victim_index() else {
            // audit: not a player-facing line
            return "no companion remains to take the merciful death".to_string(); // audit: not a player-facing line
        };
        if index == 0 {
            // audit: not a player-facing line
            return "only the Avatar remains, so Blackthorn spares the player".to_string(); // audit: not a player-facing line
        }
        // `blackthorn.md §5`: "The same execution runs on the *correct*-answer
        // branch whenever at least two travelling party members are nondead, under a different
        // message - Blackthorn thanking the player for their honesty and
        // granting the companion 'a merciful death'." Both branches therefore
        // share one execution routine.
        match self.execute_blackthorn_companion(index) {
            Some(report) => format!("{report} as a merciful death"),
            // audit: not a player-facing line
            None => "no companion remains to take the merciful death".to_string(), // audit: not a player-facing line
        }
    }

    /// `blackthorn.md §5`: "parks the lifted record in the last roster slot
    /// with a **whereabouts value that matches no location**. ... The value is
    /// `0x7F`" (u5-spec#298 Q5, `RETRACTIONS.md` R546). It matches no inn and
    /// no scene, so no inn can ever retrieve the record; it is not, as the
    /// withdrawn text said, unread: "the shrine and Codex presentation counts
    /// roster records holding it" (`karma.md` Section 8). The engine had
    /// chosen its own no-scene sentinel `0xFF` while the value was
    /// unpublished. The value is published in `formats/saved-gam.md §3.1`
    /// (row `0x1F`) and shared with the Codex scan as
    /// [`crate::INTERRED_WHEREABOUTS`].
    pub const BLACKTHORN_EXECUTED_WHEREABOUTS: u8 = crate::INTERRED_WHEREABOUTS;

    /// `blackthorn.md §5`: "**The punishment is an execution, and it is
    /// durable.** ... the routine:
    ///
    /// - clears the second-member actor's sprite (step 5 above; that actor is
    ///   not necessarily the removed companion when a Dead entry precedes it);
    /// - lifts their roster record out of the party, compacts the remaining
    ///   records up, and decrements the party count;
    /// - parks the lifted record in the last roster slot with a
    ///   **whereabouts value that matches no location**."
    ///
    /// §8 records the result as "Durable and irreversible", and §5 adds that
    /// "the refuge/rescue sequence does not restore them either. **The
    /// companion is dead and gone, and the effect survives saving and
    /// reloading.**" This is the shared routine behind both the §5 failure
    /// reaction and the §4 correct-answer "merciful death".
    ///
    /// Returns a description of what was executed, or `None` when `index`
    /// names no companion behind the Avatar.
    pub fn execute_blackthorn_companion(&mut self, index: usize) -> Option<String> {
        if index == 0 || index >= self.party.len() {
            return None;
        }

        // §5 step 5 (u5-spec#298 Q24): "Clear the second-member actor's
        // sprite" - the `SecondPartyMember` actor slot of §6, whichever member
        // is actually removed.
        let actor_slot = BlackthornCutsceneActor::SecondPartyMember.slot_index() as usize;
        if let Some(object) = self.active_objects.get_mut(actor_slot) {
            *object = ActiveObject::empty();
        }

        // "lifts their roster record out of the party, compacts the remaining
        // records up, and decrements the party count".
        let mut roster = self.synced_party_roster();
        let lifted = if index < roster.len() {
            Some(roster.remove(index))
        } else {
            None
        };
        let removed = self.party.remove(index);
        if index < self.party_names.len() {
            self.party_names.remove(index);
        }
        if index < self.party_experience.len() {
            self.party_experience.remove(index);
        }
        if index < self.party_stay_counters.len() {
            self.party_stay_counters.remove(index);
        }
        if index < self.party_strengths.len() {
            self.party_strengths.remove(index);
        }
        if index < self.party_intelligence.len() {
            self.party_intelligence.remove(index);
        }
        if index < self.party_equipment.len() {
            self.party_equipment.remove(index);
        }
        for (slot, member) in self.party.iter_mut().enumerate() {
            member.slot = slot as u8;
        }
        match self.active_player {
            Some(active) if active == index => self.active_player = None,
            Some(active) if active > index => self.active_player = Some(active - 1),
            _ => {}
        }

        // "parks the lifted record in the last roster slot with a whereabouts
        // value that matches no location". `formats/saved-gam.md` puts the
        // roster at sixteen fixed slots, so the park target is the sixteenth;
        // a shorter modelled roster parks at its own last slot.
        if let Some(record) = lifted {
            let park_slot = (SAVE_ROSTER_SLOT_COUNT - 1).min(roster.len());
            // The roster record carries its own marker byte now (u5-spec#299
            // Q14), so the parked record is written with the whereabouts too.
            let mut parked = record.clone();
            parked.inn_marker = Self::BLACKTHORN_EXECUTED_WHEREABOUTS;
            roster.insert(park_slot, parked);
            // Lodged guests are whole roster records, so the records the
            // park shifts down carry their registry slots with them.
            self.shift_inn_registry_for_roster_move(index, park_slot);
            // The whereabouts byte is the record's own inn-lodging marker,
            // its last byte `0x1F` (`formats/saved-gam.md` §3.3, u5-spec#299
            // Q14; the "shifted inn-guest view" is `RETRACTIONS.md` R569).
            // Writing an unmatchable marker there is what makes the record
            // unretrievable by any inn while still surviving save/reload.
            if self.inn_registry.len() < INN_REGISTRY_CAP {
                // Registry slot `k`'s marker is roster record `k`'s byte
                // `0x1F` (`formats/saved-gam.md §3.1`), so the parked record's
                // whereabouts sits at the park slot when that slot is free -
                // the byte `karma.md §8`'s scan of roster slots 1..15 reads
                // (u5-spec#298 Q5).
                let registry_slot = u8::try_from(park_slot)
                    .ok()
                    .filter(|slot| {
                        usize::from(*slot) < SAVE_INN_REGISTRY_COUNT
                            && !self
                                .inn_registry
                                .iter()
                                .any(|guest| guest.registry_slot == *slot)
                    })
                    .or_else(|| crate::free_inn_registry_slot(&self.inn_registry))
                    .unwrap_or_default();
                self.inn_registry.push(InnGuestRecord {
                    registry_slot,
                    scene_marker: Self::BLACKTHORN_EXECUTED_WHEREABOUTS,
                    name: record.name,
                    member: record.member,
                    strength: record.strength,
                    intelligence: record.intelligence,
                    experience: record.experience,
                    equipment: record.equipment,
                    stay_counter: record.stay_counter,
                });
            }
        }
        self.party_roster = roster;
        self.mark_visibility_dirty();

        Some(format!(
            "roster slot {} is executed; party count {}",
            removed.slot + 1,
            self.party.len()
        ))
    }

    pub fn apply_blackthorn_captive_cell_handoff(
        &mut self,
        game_dir: &Path,
        prefix: &str,
    ) -> io::Result<MoveOutcome> {
        let scene = Scene::new(BLACKTHORN_CAPTIVE_CELL_SCENE)?;
        let floor = crate::blackthorn::BLACKTHORN_CAPTIVE_CELL_FLOOR;
        let (grid, beacon_sources) =
            load_town_runtime_floor_with_beacon_sources(game_dir, scene, floor, self.clock.hour)?;
        self.grid = grid;
        // `visibility.md §12.6`: a new location floor is fresh map setup —
        // clear both beacon positions and re-record up to two bright-light
        // hits. Harvested from the RAW floor, because the runtime
        // normalisation pass scrubs the marker byte the beacon looks for.
        self.light_beacon.sources = beacon_sources;
        self.natural_moongate_live_cells.clear();
        self.area = Area::Town { scene, floor };
        self.player.x = BLACKTHORN_CAPTIVE_CELL_X as usize;
        self.player.y = BLACKTHORN_CAPTIVE_CELL_Y as usize;
        self.player.transport = TransportState::Foot;
        self.pending_town_arrest = None;
        self.active_blackthorn = None;
        self.blackthorn_audience_map = None;
        self.blackthorn_capture = None;
        // `blackthorn.md §8`: "Carried-key count zeroed by the audience
        // cleanup | Durable inventory effect of the capture". The byte an
        // earlier revision of that table called a Blackthorn conversation
        // signal "is the party's ordinary carried-key counter ... and the
        // audience's cleanup simply zeroes it, so the party leaves the
        // capture without its keys". This handoff is that cleanup, so the
        // debit fires on every exit path out of the interrogation.
        self.keys = 0;
        self.clear_town_floor_reload_door_state();
        let tlk = parse_tlk(&game_dir.join(format!("{}.TLK", scene.family.stem())))?;
        let npc_slots = parse_npc_block(game_dir, scene, &tlk)?;
        self.load_scheduled_npcs(&npc_slots);
        let _ = self.restore_resident_shadowlord_after_floor_reload();
        self.sync_player_object();
        self.mark_visibility_dirty();
        // The location report is harness detail: `blackthorn.md §4.1` says
        // "Do not append an answer, prompt ordinal, roster slot number,
        // cutscene timing value or other diagnostic information to the
        // original text", and the handoff's caller supplies that text.
        self.push_diagnostic(format!(
            "Returned to Blackthorn's captive cell in {} at ({}, {}).",
            scene.key(),
            self.player.x,
            self.player.y
        ));
        self.message = prefix.to_string();
        Ok(MoveOutcome::Transition(AreaTransition::EnteredLocation(
            scene,
        )))
    }

    /// `blackthorn.md §7`'s rescue cinematic, steps 1 to 18.
    ///
    /// The narration is staged rather than printed in one call: §7's
    /// "complete print-and-wait order" interleaves nine beats with BIOS-tick
    /// waits, and step 19 is "the cinematic's only blocking key read". This
    /// call runs everything up to that key; [`Self::step_blackthorn_rescue`]
    /// owns the rest.
    ///
    /// The engine used to print the `KARMA.DAT` verdict and nothing else,
    /// because §7's contract list names the beats only by role. The literals
    /// are published in the section's beat table, which also states that none
    /// of them is a data-file record: "An implementation cannot read these
    /// strings out of the user's data files; it has to carry them."
    pub fn apply_blackthorn_rescue_refuge(&mut self, game_dir: &Path) -> io::Result<MoveOutcome> {
        // The deaths that brought the party here have not reached the panel
        // on every route into this cinematic: Stonegate's script declines the
        // step's own refresh so its 750-tone sweep runs with the pre-death
        // roster still on screen (`audio.md §8.2`, and the note in the
        // trapdoor arm). This is the other side of that - the sweep is over,
        // the rescue owns the screen, and the roster is repainted.
        //
        // Measured 2026-09-18 (`qa/paired/stonegate-rescue-pacing.tsv`): the
        // original's roster row turns to `0D` at `t28`, the first sample
        // after the sweep, which is this beat.
        self.repaint_stats_panel();
        let verdict = blackthorn_rescue_verdict_record(self.moral_standing);
        let verdict_message = self.blackthorn_rescue_verdict_message(game_dir, verdict as usize)?;

        // Step 1-2: "Wait ten BIOS ticks. Print beat 1." Each hold below
        // also carries the measured untimed work beside its wait; see
        // [`crate::blackthorn::BLACKTHORN_RESCUE_UNTIMED_BEFORE_DARKNESS`].
        self.staged_narration.push_ticks(
            crate::blackthorn::BLACKTHORN_RESCUE_WAIT_BEFORE_DARKNESS
                + crate::blackthorn::BLACKTHORN_RESCUE_UNTIMED_BEFORE_DARKNESS,
            crate::blackthorn::BLACKTHORN_RESCUE_DARKNESS,
        );

        // Step 3: "Dissolve the map viewport out to black; clear the
        // cinematic scratch state." `blackthorn.md §7` puts this before
        // scratch clearing, tableau construction and every durable write.
        self.run_map_viewport_dissolve(MapViewportDissolveSource::BlackthornRescueBlack);

        // Steps 4-8: the three refuge fragments, fourteen then twenty-eight
        // ticks apart.
        // The dissolve above costs no time in the model, so the refuge
        // fragments used to follow the darkness beat immediately. See
        // [`crate::BLACKTHORN_RESCUE_WAIT_VIEWPORT_DISSOLVE`].
        self.staged_narration.push_ticks(
            crate::blackthorn::BLACKTHORN_RESCUE_WAIT_VIEWPORT_DISSOLVE
                + crate::blackthorn::BLACKTHORN_RESCUE_UNTIMED_AROUND_DISSOLVE,
            crate::blackthorn::BLACKTHORN_RESCUE_REFUGE_FIRST,
        );
        self.staged_narration.push_ticks(
            crate::blackthorn::BLACKTHORN_RESCUE_WAIT_BEFORE_REFUGE_SECOND,
            crate::blackthorn::BLACKTHORN_RESCUE_REFUGE_SECOND,
        );
        self.staged_narration.push_ticks(
            crate::blackthorn::BLACKTHORN_RESCUE_WAIT_BEFORE_REFUGE_THIRD
                + crate::blackthorn::BLACKTHORN_RESCUE_UNTIMED_BEFORE_ENVELOPES,
            crate::blackthorn::BLACKTHORN_RESCUE_REFUGE_THIRD,
        );

        // Step 9: "Install the party-on-foot actor at the viewport centre and
        // redraw; run the six-row envelope sequence of Section 7.1."
        //
        // `blackthorn.md §7.1`: the three direct cell reveals all use the
        // shared 256-pixel LFSR order and 31 checkpoints, and the thunder
        // beat invokes the shared major flash twice, consuming 3,712
        // gameplay-PRNG draws even when sound is muted.
        self.pending_blackthorn_rescue_playbacks
            .push(blackthorn_rescue_playback());
        // `audio.md §8.6.2`: after the refuge tableau first redraws the party
        // actor, run six independent envelope programs back-to-back. No visual
        // operation or intentional hold occurs between rows. They run when
        // the third refuge fragment is released, not now: emitting them here
        // played them under the darkness beat and the refuge narration
        // (playtest N16). See [`Self::release_blackthorn_rescue_beat_audio`].

        // Steps 10-14: the shout, then six plus four plus four ticks across
        // the two Guardian reveals, then the thunder line.
        self.staged_narration
            .push_ticks(
                crate::blackthorn::BLACKTHORN_RESCUE_WAIT_ENVELOPE_SEQUENCE,
                crate::blackthorn::BLACKTHORN_RESCUE_SHOUT,
            );
        // The two Guardian reveals are blocking in the original and free
        // here. See [`crate::BLACKTHORN_RESCUE_WAIT_CELL_REVEAL`].
        self.staged_narration.push_ticks(
            crate::blackthorn::BLACKTHORN_RESCUE_WAIT_BEFORE_THUNDER
                + 2 * crate::blackthorn::BLACKTHORN_RESCUE_WAIT_CELL_REVEAL,
            crate::blackthorn::BLACKTHORN_RESCUE_THUNDER,
        );
        // Step 15: the paired viewport flash runs once the thunder line has
        // printed; see [`Self::release_blackthorn_rescue_beat_audio`].

        // Steps 16-18: `\n"`, the record, one `"`. One emission, because the
        // frame and the record share a row.
        //
        // Held for the paired flash above, which `audio.md §8.4` measures at
        // about 0.86 s per invocation on this harness's own emulator. See
        // [`crate::BLACKTHORN_RESCUE_WAIT_PAIRED_FLASH`].
        // `§7.1` step 5: the third reveal, the crowned figure, follows the
        // flash and precedes the verdict.
        self.staged_narration.push_ticks(
            crate::blackthorn::BLACKTHORN_RESCUE_WAIT_PAIRED_FLASH
                + crate::blackthorn::BLACKTHORN_RESCUE_WAIT_CELL_REVEAL,
            format!(
                "{}{verdict_message}{}",
                crate::blackthorn::BLACKTHORN_RESCUE_VERDICT_OPEN,
                crate::blackthorn::BLACKTHORN_RESCUE_VERDICT_CLOSE,
            ),
        );

        self.pending_blackthorn_rescue = Some(crate::blackthorn::BlackthornRescuePhase::Narration);
        Ok(MoveOutcome::Observed)
    }

    /// The rescue's audio, run as the beat it follows is released.
    ///
    /// `blackthorn.md §7`'s print-and-wait order: "8. Print beat 4. 9. Install
    /// the party-on-foot actor at the viewport centre and redraw; run the
    /// six-row envelope sequence of Section 7.1", and `§7.1` item 4: "Print
    /// the thunder line, then run the shared full-viewport flash/low-rumble
    /// effect twice consecutively". `audio.md §2`: an effect owns the calling
    /// thread, so the narration around it cannot advance underneath it. The
    /// cinematic used to emit all of it in one batch as it started, so the
    /// envelopes sounded under the darkness beat and the refuge fragments and
    /// the flashes about nine seconds before the thunder line (playtest N16).
    /// The holds that follow each release already carry the effects' length
    /// ([`crate::blackthorn::BLACKTHORN_RESCUE_WAIT_ENVELOPE_SEQUENCE`],
    /// [`crate::blackthorn::BLACKTHORN_RESCUE_WAIT_PAIRED_FLASH`]).
    pub(crate) fn release_blackthorn_rescue_beat_audio(&mut self, text: &str) {
        if self.pending_blackthorn_rescue.is_none() {
            return;
        }
        if text == crate::blackthorn::BLACKTHORN_RESCUE_REFUGE_THIRD {
            self.emit_sound_effect(SoundEffect::BlackthornRescueEnvelopes);
        } else if text == crate::blackthorn::BLACKTHORN_RESCUE_THUNDER {
            self.emit_major_flash();
            self.emit_major_flash();
        }
    }

    /// One world tick of the rescue figure's animator while the verdict's
    /// key wait runs. `endgame.md §4` (issue #298 Q43): "The Blackthorn
    /// rescue's crowned spectral figure (`0x74`, also script A) likewise
    /// animates during its verdict key wait" - `blackthorn.md §7` step 19:
    /// "The world keeps running while it blocks". Returns whether the
    /// displayed frame changed.
    pub fn animate_blackthorn_rescue_spectral(&mut self) -> bool {
        if !self.blackthorn_rescue_awaiting_acknowledgement()
            || self.blackthorn_rescue_spectral_actor.is_empty()
        {
            return false;
        }
        let mut figure = self.blackthorn_rescue_spectral_actor;
        let changed = self.animate_script_a_object_tick(&mut figure);
        self.blackthorn_rescue_spectral_actor = figure;
        changed
    }

    /// Run [`Self::animate_blackthorn_rescue_spectral`] once for every BIOS
    /// tick since the last call - the ordinary reader's idle arm ticks the
    /// world at that rate - catching up at most
    /// [`crate::ENDGAME_TABLEAU_ANIMATION_CATCH_UP_TICKS`] at once.
    pub fn advance_blackthorn_rescue_animation_to(&mut self, now_secs: f64) -> bool {
        if !self.blackthorn_rescue_awaiting_acknowledgement() {
            return false;
        }
        let tick = (now_secs.max(0.0) / crate::narration::BIOS_TICK_SECS) as u64;
        let Some(last) = self.blackthorn_rescue_animation_tick.replace(tick) else {
            return false;
        };
        let due = tick
            .saturating_sub(last)
            .min(crate::ENDGAME_TABLEAU_ANIMATION_CATCH_UP_TICKS);
        let mut changed = false;
        for _ in 0..due {
            changed |= self.animate_blackthorn_rescue_spectral();
        }
        changed
    }

    /// The tile the rescue's crowned figure is drawn with: `0x100` plus the
    /// animated record's frame byte (`endgame.md §4`'s `+256` rule), or the
    /// revealed atlas tile `0x174` before the record goes live.
    pub fn blackthorn_rescue_spectral_atlas_index(&self) -> u16 {
        if self.blackthorn_rescue_spectral_actor.is_empty() {
            crate::blackthorn::BLACKTHORN_RESCUE_SPECTRAL_ATLAS_INDEX
        } else {
            0x100 + u16::from(self.blackthorn_rescue_spectral_actor.tile)
        }
    }

    /// The audience actor in `slot`, as `(column, row, effective tile byte,
    /// map terrain, previous-row terrain, next-row terrain)`, or `None` when
    /// the slot holds no cutscene actor on the grid.
    fn blackthorn_audience_actor_cell(
        &self,
        slot: usize,
    ) -> Option<(usize, usize, ActiveObject, u8, Option<u8>, Option<u8>)> {
        let map = self.blackthorn_audience_map.as_ref()?;
        let object = *self.active_objects.get(slot)?;
        if object.is_empty()
            || object.aux3 != BLACKTHORN_CUTSCENE_AUX3_ROLE_MARKER
            // `blackthorn.md §3` (u5-spec#299 Q11): the further party
            // prisoners in slots 2 to 5 are drawn like the named actors.
            || (crate::blackthorn::blackthorn_cutscene_actor(slot as u8).is_none()
                && !crate::blackthorn::BLACKTHORN_AUDIENCE_FURTHER_PARTY_SLOTS.contains(&slot))
        {
            return None;
        }
        let terrain = map.tile(object.x, object.y)?;
        let previous = object.y.checked_sub(1).and_then(|y| map.tile(object.x, y));
        let next = map.tile(object.x, object.y + 1);
        Some((object.x, object.y, object, terrain, previous, next))
    }

    /// The full atlas index the audience viewport draws for the actor in
    /// `slot`, or `None` to draw nothing there.
    ///
    /// `blackthorn.md §6`'s explicit redraw is "one unconditional world
    /// tick", so the audience's actors go through the shared `visibility.md
    /// §8` compositor like any world actor: every placed party member (§3
    /// "Party placement", class tile per member, u5-spec#299 Q11), the
    /// Avatar at `(3,5)` and the second member at `(7,5)` included, starts
    /// on the manacles `0x85` and is drawn as "one of `0x64..0x67`" - the
    /// prisoner set. Stock capture (`bt-correct`, every audience beat) shows
    /// exactly that prisoner on both cells. The draw-nothing sentinel `0x16`
    /// (§6.1) still draws nothing.
    pub fn blackthorn_audience_actor_atlas_tile(&self, slot: usize) -> Option<usize> {
        let (_, row, object, terrain, previous, next) =
            self.blackthorn_audience_actor_cell(slot)?;
        if object.tile == crate::ACTOR_TILE_TRANSPARENT_BYTE {
            return None;
        }
        let variant = self
            .blackthorn_audience_actor_variants
            .get(slot)
            .copied()
            .unwrap_or(0);
        let tile = match active_object_composite(
            object.type_byte,
            object.tile,
            terrain,
            terrain,
            previous,
            next,
            row,
            variant,
        ) {
            ActiveObjectCompositeResult::Suppress | ActiveObjectCompositeResult::Direct(_) => {
                return None
            }
            ActiveObjectCompositeResult::Companion(tile)
            | ActiveObjectCompositeResult::PreviousRowDirectAndCompanion { tile, .. } => tile,
        };
        crate::actor_tile_for_byte(tile)
    }

    /// One composite pass over the audience actors: every actor that lands
    /// on a selecting row takes a fresh variant draw (`visibility.md §8.1`:
    /// "once per composite pass ... never cached"), walking slot eight down
    /// to slot zero; every other actor takes none. Returns whether any drawn
    /// variant changed.
    pub fn run_blackthorn_audience_composite_pass(&mut self) -> bool {
        let mut changed = false;
        for slot in (0..self.blackthorn_audience_actor_variants.len()).rev() {
            let Some((_, _, object, terrain, previous, next)) =
                self.blackthorn_audience_actor_cell(slot)
            else {
                continue;
            };
            if !composite_active_object_slot_draws_variant(
                object.type_byte,
                object.tile,
                terrain,
                terrain,
                previous,
                next,
            ) {
                continue;
            }
            let variant = self.draw_active_object_composite_variant();
            changed |= self.blackthorn_audience_actor_variants[slot] != variant;
            self.blackthorn_audience_actor_variants[slot] = variant;
        }
        changed
    }

    /// Run [`Self::run_blackthorn_audience_composite_pass`] once for every
    /// BIOS tick since the last call while the audience map is up - the
    /// world-tick rate of the audience's pauses and key waits - catching up
    /// at most [`crate::ENDGAME_TABLEAU_ANIMATION_CATCH_UP_TICKS`] at once.
    pub fn advance_blackthorn_audience_composite_to(&mut self, now_secs: f64) -> bool {
        if self.blackthorn_audience_map.is_none() {
            self.blackthorn_audience_composite_tick = None;
            return false;
        }
        let tick = (now_secs.max(0.0) / crate::narration::BIOS_TICK_SECS) as u64;
        let Some(last) = self.blackthorn_audience_composite_tick.replace(tick) else {
            return self.run_blackthorn_audience_composite_pass();
        };
        let due = tick
            .saturating_sub(last)
            .min(crate::ENDGAME_TABLEAU_ANIMATION_CATCH_UP_TICKS);
        let mut changed = false;
        for _ in 0..due {
            changed |= self.run_blackthorn_audience_composite_pass();
        }
        changed
    }

    /// Whether the rescue is at step 19, its one blocking key read.
    /// Whether `blackthorn.md §7`'s rescue cinematic owns the viewport.
    pub fn blackthorn_rescue_active(&self) -> bool {
        self.pending_blackthorn_rescue.is_some()
    }

    pub fn blackthorn_rescue_awaiting_acknowledgement(&self) -> bool {
        self.pending_blackthorn_rescue
            == Some(crate::blackthorn::BlackthornRescuePhase::AwaitingAcknowledgement)
    }

    /// Drive the rescue past whatever it is waiting on.
    ///
    /// Called once per frame by the shell. Returns the transition when the
    /// cinematic reaches its handoff, and `None` every other time.
    pub fn step_blackthorn_rescue(
        &mut self,
        game_dir: &Path,
        key_pressed: bool,
    ) -> io::Result<Option<MoveOutcome>> {
        use crate::blackthorn::BlackthornRescuePhase;
        match self.pending_blackthorn_rescue {
            None => Ok(None),
            // The staged queue is still printing; nothing to do until it
            // drains. §7: every beat before step 19 "passes without input".
            Some(BlackthornRescuePhase::Narration) if self.staged_narration_active() => Ok(None),
            Some(BlackthornRescuePhase::Narration) => {
                self.pending_blackthorn_rescue =
                    Some(BlackthornRescuePhase::AwaitingAcknowledgement);
                // `blackthorn.md §7.1` step 5 committed actor byte `0x74` at
                // `(5,2)`; `endgame.md §4` (issue #298 Q43): that crowned
                // figure is script A and "likewise animates during its
                // verdict key wait".
                let (x, y) = crate::blackthorn::BLACKTHORN_RESCUE_SPECTRAL_CELL;
                self.blackthorn_rescue_spectral_actor = ActiveObject {
                    type_byte: crate::blackthorn::BLACKTHORN_RESCUE_SPECTRAL_ACTOR_BYTE,
                    tile: crate::blackthorn::BLACKTHORN_RESCUE_SPECTRAL_ACTOR_BYTE,
                    x: usize::from(x),
                    y: usize::from(y),
                    z: 0,
                    phase: 0,
                    aux1: 0,
                    aux3: 0,
                };
                self.blackthorn_rescue_animation_tick = None;
                Ok(None)
            }
            // Step 19: "any key satisfies it, no typed character is echoed,
            // and the returned key is discarded rather than interpreted as a
            // command".
            Some(BlackthornRescuePhase::AwaitingAcknowledgement) => {
                if !key_pressed {
                    return Ok(None);
                }
                // Step 20: beat 8, which carries no trailing feed.
                self.staged_narration
                    .push_ticks(0, crate::blackthorn::BLACKTHORN_RESCUE_INTONED);
                // Steps 21-22: four ticks, then the restoration loop, which
                // "prints beat 8a once per non-Dead in-party slot". The status
                // read is the pre-restoration one - §7 notes the beat is
                // unreachable on an all-Dead roster, which is what the
                // cinematic's own entry condition normally leaves. The loop
                // itself runs below, one staged step per slot, so each slot's
                // panel repaint lands after its own beat.
                let mut wait = crate::blackthorn::BLACKTHORN_RESCUE_WAIT_AFTER_INTONED;
                // Step 22's durable half. `RETRACTIONS.md` R493 withdraws
                // **both halves** of the sentence this used to quote - "the
                // member's status is reset to able-bodied and their current
                // hit points are set to their maximum" - and the loop that
                // implemented it: "the restore is neither unconditional nor
                // lossless".
                //
                // What it is: "the cinematic dispatches the shared revive
                // routine per in-party slot, and that routine acts **only** on
                // a slot whose stored status is Dead", setting able-bodied,
                // one hit point, mana from the class letter, and - below a
                // moral standing of ninety-eight - a rescaled experience, a
                // recomputed level and a recomputed maximum. "Only then does
                // the cinematic copy that newly recomputed maximum into
                // current hit points."
                //
                // So a party wipe **costs experience in proportion to the
                // party's karma**, and members can return at a lower level
                // with a lower maximum than they had before dying. An engine
                // built on the old sentence silently preserves progression the
                // original destroys.
                //
                // A slot that is not Dead "is left entirely alone by the
                // revive routine and prints `Not dead!` instead, though the
                // cinematic's own current-equals-maximum copy still heals it
                // to full" - which is why that copy runs over every slot below
                // and the revive dispatch does not.
                // §7 "Party restoration": each slot's copy is followed by
                // "repaints the whole stats panel" - the immediate repaint of
                // `stats-panel.md §2.2`, once per restored member. The
                // cinematic runs outside the command dispatcher's snapshot
                // test, so nothing else repainted it and the panel kept the
                // dead roster past the handoff (playtest E59). The painted
                // body is staged so a paced frontend shows it at the slot's
                // own step rather than during beat 8.
                self.stats_panel_hold = Some(self.displayed_stats_panel().clone());
                let mut not_dead_continues_intoned = true;
                for slot in 0..self.party.len() {
                    // `audio.md §8.6.2` / R494: one envelope per in-party
                    // slot, in ascending slot order, immediately before that
                    // slot's restore dispatch.
                    self.emit_sound_effect(crate::audio::SoundEffect::BlackthornRescueRestoration(
                        slot,
                    ));
                    if self.party[slot].status == b'D' {
                        self.resurrect_party_member_to_hp(slot, 1);
                    } else if std::mem::take(&mut not_dead_continues_intoned) {
                        // §7: beat 8 "has **no** trailing line feed, so when
                        // beat 8a fires the first `Not dead!` runs straight
                        // on from `intoned.` on the same row" (playtest H48).
                        // The panel-repaint holds between them print nothing,
                        // so the row is still the one beat 8 left open.
                        self.staged_narration.push_secs_on_open_row(
                            crate::narration::bios_ticks_secs(wait),
                            crate::blackthorn::BLACKTHORN_RESCUE_NOT_DEAD,
                        );
                        wait = 0;
                    } else {
                        self.staged_narration
                            .push_ticks(wait, crate::blackthorn::BLACKTHORN_RESCUE_NOT_DEAD);
                        wait = 0;
                    }
                    // `blackthorn.md §7` "Party restoration": per slot the
                    // cinematic revives, "copies the member's maximum hit
                    // points into their current hit points, and repaints the
                    // whole stats panel, before moving to the next slot" -
                    // the immediate repaint of `stats-panel.md §2.2`. Without
                    // it the panel kept the rescue's `0D` roster (playtest
                    // E23, E59).
                    self.party[slot].hp = self.party[slot].max_hp.max(1);
                    self.repaint_stats_panel();
                    self.staged_stats_panels.push_back(self.stats_panel.clone());
                    self.staged_narration
                        .push_secs_repainting_panel(crate::narration::bios_ticks_secs(wait));
                    wait = 0;
                }
                // Step 23: beat 9. The four-tick wait of step 21 went
                // to the first slot's step above.
                self.staged_narration
                    .push_ticks(wait, crate::blackthorn::BLACKTHORN_RESCUE_VERTIGO);
                // Step 24: four ticks before the handoff, with nothing to
                // print - an empty beat is a pure hold.
                self.staged_narration.push_ticks(
                    crate::blackthorn::BLACKTHORN_RESCUE_WAIT_AFTER_VERTIGO,
                    String::new(),
                );
                self.pending_blackthorn_rescue = Some(BlackthornRescuePhase::Restoration);
                Ok(None)
            }
            Some(BlackthornRescuePhase::Restoration) if self.staged_narration_active() => Ok(None),
            Some(BlackthornRescuePhase::Restoration) => {
                self.pending_blackthorn_rescue = None;
                self.apply_blackthorn_rescue_handoff(game_dir).map(Some)
            }
        }
    }

    /// Finish `blackthorn.md §4.2`'s audience exit beat once its hold has
    /// run out, performing the captive-cell handoff it precedes.
    ///
    /// Returns the transition on the frame the beat ends, and `None`
    /// otherwise.
    pub fn step_blackthorn_audience_exit(
        &mut self,
        game_dir: &Path,
    ) -> io::Result<Option<MoveOutcome>> {
        if !self.pending_blackthorn_audience_exit || self.staged_narration_active() {
            return Ok(None);
        }
        self.pending_blackthorn_audience_exit = false;
        // The exit vector has cleared the tableau slot, so this gate skips.
        self.run_blackthorn_conditional_throne_cleanup();
        self.apply_blackthorn_captive_cell_handoff(game_dir, "")
            .map(Some)
    }

    /// Run the exit beat straight through, for shells with no pump.
    pub fn run_blackthorn_audience_exit_to_handoff(
        &mut self,
        game_dir: &Path,
    ) -> io::Result<Option<MoveOutcome>> {
        if !self.pending_blackthorn_audience_exit {
            return Ok(None);
        }
        self.flush_staged_narration();
        self.step_blackthorn_audience_exit(game_dir)
    }

    /// Run a rescue cinematic straight through to its handoff.
    ///
    /// For frontends with no animation pump - the terminal shell and the
    /// headless harness - and for tests. The staged beats are printed at
    /// once and step 19's blocking key is treated as already pressed, so
    /// every durable effect of `blackthorn.md §7` lands in one call.
    pub fn run_blackthorn_rescue_to_handoff(
        &mut self,
        game_dir: &Path,
    ) -> io::Result<Option<MoveOutcome>> {
        // One iteration per phase transition, with a bound so a future phase
        // that forgets to advance fails loudly instead of hanging a shell.
        for _ in 0..16 {
            if self.pending_blackthorn_rescue.is_none() {
                return Ok(None);
            }
            self.flush_staged_narration();
            let key = self.blackthorn_rescue_awaiting_acknowledgement();
            if let Some(outcome) = self.step_blackthorn_rescue(game_dir, key)? {
                return Ok(Some(outcome));
            }
        }
        Err(io::Error::other(
            "the Blackthorn rescue cinematic did not reach its handoff",
        ))
    }

    /// Step 25 and contract steps 9 to 12: the second dissolve, then every
    /// durable write. `blackthorn.md §7`: "The moral-standing floor,
    /// destination scene/floor/position writes, timed-effect clear, clock
    /// spin to hour 6 (step 11), and light-counter clears all occur only after
    /// the second dissolve has completed."
    fn apply_blackthorn_rescue_handoff(&mut self, game_dir: &Path) -> io::Result<MoveOutcome> {
        // Its hidden viewport is black except for the on-foot party tile at
        // the centre cell; the castle view is not this dissolve's source.
        self.run_map_viewport_dissolve(MapViewportDissolveSource::BlackthornRescuePartyOnBlack {
            cell: BLACKTHORN_RESCUE_PARTY_CELL,
        });

        // The cinematic is over; the handoff's own scene is an ordinary map.
        self.cutscene_black_viewport = false;
        self.blackthorn_rescue_reveals_shown = 0;
        self.blackthorn_rescue_spectral_actor = ActiveObject::empty();
        self.blackthorn_rescue_animation_tick = None;
        self.moral_standing = blackthorn_rescue_post_print_standing(self.moral_standing);
        self.clear_active_effect_slot();
        // `blackthorn.md §7` step 11 (u5-spec#299 Q17, `RETRACTIONS.md`
        // R571): "the clock then spun forward by the ordinary clock advance in
        // nine-minute steps until the hour is 6. The hour is checked before
        // each step and the minute is never reset." The timed-effect slot was
        // cleared just above, "so Quickness and Negate Time never alter a
        // step: every step is exactly nine minutes". An entry hour of 6 makes
        // no step; 7 to 23 crosses midnight into the next day. §7.1: "Once the
        // clock has been spun forward it also zeroes both light counters";
        // §8.1 puts the food repair after the spin.
        //
        // The ordinary clock half carries the rest of the step: "A midnight
        // crossing runs the ordinary day rollover once, which repaints the
        // whole stats panel straight away, so the date row shows the new
        // date" (playtest F57), "the ordinary midnight events ... including
        // the Shadowlord relocation roll", the month-boundary events, and the
        // hour-change bundle per hour crossed. It runs no provision pass and
        // no world tick, and §8.1 preserves every nonzero food word.
        //
        // Issue #299 Q28 publishes what each step does: the spin is made of
        // ordinary clock advances, so it runs "the clock advance's midnight
        // work once, and nothing beyond it" - the Shadowlord hideout reroll
        // with its random draws, the day advance, the month-boundary bundle
        // when the day passes 28, then "the single panel repaint". "Each hour
        // crossed also lowers the camp cooldown by one, floored at zero ...
        // and rewrites the cached twelve-hour value", each step recomputes
        // daylight in the castle, and "After a spin that stepped, the shared
        // previous-hour snapshot ... reads 5 against hour 6". "The spin does
        // not run the party status and provision pass ..., the world tick or
        // active objects, NPC schedule work, natural-moongate work or
        // encounters", and "An entry hour of 6 runs none of this work". That
        // is exactly this engine's clock half of a turn
        // ([`Self::advance_clock_half`]), stepped nine minutes at a time.
        // Its light-counter decay is moot: both counters are zeroed straight
        // after.
        //
        // The withdrawn reading - "spun forward until the hour reads 06:00"
        // - stepped whole hours here, which kept the entry minute instead of
        // landing at 06:00-06:08.
        //
        // The spin runs in the castle scene: "Each hour crossed also runs the
        // ordinary hour-change bundle for the castle scene", and the midnight
        // reroll's "party-scene rejection is inert, because the scene is
        // already the castle" (u5-spec#299 Q28). So the scene is written
        // before the spin, and the reroll no longer rejects the town the
        // party was wiped in; the floor itself is loaded below, at the spun
        // hour. §7 step 11: "on logical floor one" - the signed floor `+1`.
        let scene = Scene::new(BLACKTHORN_RESCUE_HANDOFF_SCENE)?;
        let floor = crate::blackthorn::BLACKTHORN_RESCUE_HANDOFF_FLOOR;
        self.area = Area::Town { scene, floor };
        while self.clock.hour != crate::blackthorn::BLACKTHORN_RESCUE_HANDOFF_HOUR {
            self.advance_clock_half(crate::blackthorn::BLACKTHORN_RESCUE_CLOCK_STEP_MINUTES);
        }
        self.torch_counter = 0;
        self.light_spell_counter = 0;
        // `blackthorn.md §8.1` (R284): there is no captive-cell counter.
        // Rescue instead initializes the existing Food word only when the
        // complete little-endian value is zero; every nonzero value survives.
        if self.food == 0 {
            self.food = 63;
        }
        let (grid, beacon_sources) =
            load_town_runtime_floor_with_beacon_sources(game_dir, scene, floor, self.clock.hour)?;
        self.grid = grid;
        // `visibility.md §12.6`: a new location floor is fresh map setup —
        // clear both beacon positions and re-record up to two bright-light
        // hits. Harvested from the RAW floor, because the runtime
        // normalisation pass scrubs the marker byte the beacon looks for.
        self.light_beacon.sources = beacon_sources;
        self.natural_moongate_live_cells.clear();
        self.area = Area::Town { scene, floor };
        self.player.x = BLACKTHORN_RESCUE_HANDOFF_X as usize;
        self.player.y = BLACKTHORN_RESCUE_HANDOFF_Y as usize;
        self.force_foot_transport();
        self.pending_town_arrest = None;
        self.active_blackthorn = None;
        self.blackthorn_audience_map = None;
        self.clear_town_floor_reload_door_state();
        let tlk = parse_tlk(&game_dir.join(format!("{}.TLK", scene.family.stem())))?;
        let npc_slots = parse_npc_block(game_dir, scene, &tlk)?;
        self.load_scheduled_npcs(&npc_slots);
        let _ = self.restore_resident_shadowlord_after_floor_reload();
        self.sync_player_object();
        self.mark_visibility_dirty();
        // Scene, coordinate, standing and restoration details are runtime
        // state, not player-facing prose.
        self.push_diagnostic(format!(
            "Rescued to {} at ({}, {}).",
            scene.key(),
            self.player.x,
            self.player.y
        ));
        Ok(MoveOutcome::Transition(AreaTransition::EnteredLocation(
            scene,
        )))
    }

    pub fn take_pending_blackthorn_rescue_playbacks(&mut self) -> Vec<BlackthornRescuePlayback> {
        std::mem::take(&mut self.pending_blackthorn_rescue_playbacks)
    }

    pub fn blackthorn_rescue_verdict_message(
        &self,
        game_dir: &Path,
        record_index: usize,
    ) -> io::Result<String> {
        let Some(records) = load_karma_records(game_dir)? else {
            return Ok(String::new());
        };
        Ok(records.get(record_index).cloned().unwrap_or_default())
    }

    pub fn apply_dungeon_post_turn_effects_after_turn(
        &mut self,
        turn_before: u64,
        game_dir: &Path,
    ) -> io::Result<Option<MoveOutcome>> {
        if self.turn == turn_before {
            return Ok(None);
        }
        let Area::Dungeon { scene, level } = self.area else {
            return Ok(None);
        };
        // `dungeon-mode.md §4` step 6 (issue #299 Q23): the post-action
        // helper's "first act is the dungeon wake roll: each Sleeping member
        // independently wakes (becomes Good) with probability 1 in 16 (a
        // uniform roll over the inclusive range 0..63, waking below 4), and
        // the stats panel is repainted if anyone woke." It is the dungeon's
        // only per-turn wake rule and needs an acted command, so the
        // all-asleep cycle of step 7 never reaches it.
        let mut woke = false;
        for index in 0..self.party.len() {
            if self.party[index].status == b'S'
                && u5_prng_range_u16(
                    &mut self.prng_state,
                    0,
                    u16::from(DUNGEON_SLEEP_WAKE_ROLL_MAX),
                ) < u16::from(DUNGEON_SLEEP_WAKE_BELOW)
            {
                self.party[index].status = b'G';
                woke = true;
            }
        }
        if woke {
            self.repaint_stats_panel();
        }
        let x = self.player.x;
        let y = self.player.y;
        let tile = self.dungeon_cell(level, x, y);
        if let Some(entry) = self.dungeon_teleport_at(Some(game_dir), scene, level, x, y, tile)? {
            // `doors-and-z-transitions.md` (dungeon level <-> dungeon
            // level): the scripted teleport is a post-action level change,
            // and its report is a row of its own under the command's result,
            // never one sentence with it - the same row model as the world
            // path's plane transitions
            // ([`Self::keep_post_turn_effect_rows_separate`]). The teleport
            // writes the slot directly rather than through
            // [`Self::emit_message_line`], so the command's own row is
            // flushed here first; otherwise the assignment would drop it.
            let pre_effect_message = self.message.clone();
            let pre_effect_row_open = self.message_row_open_mid_line;
            self.flush_message_slot();
            let outcome = self.apply_dungeon_teleport_after_turn(scene, entry);
            self.keep_post_turn_effect_rows_separate(&pre_effect_message, pre_effect_row_open);
            if self.message != pre_effect_message && !self.message.is_empty() {
                // The teleport's report is one complete row; nothing it
                // printed leaves the cursor mid-line. Set explicitly - the
                // direct slot write above sets no row state of its own.
                self.message_row_open_mid_line = false;
            }
            return Ok(Some(outcome));
        }
        if is_dungeon_fall_trap(tile) {
            // `dungeon-mode.md §8.1`: the pit group (`Pit Trap!`,
            // `Falling...`, `      ...splat!`) goes out through
            // [`Self::emit_message_line`], which flushes the command's row
            // first and sets the row state from each line it logs. The
            // slot is never re-joined to the command's text; an off-bottom
            // fall that prints nothing restores the command's row and the
            // row state the command set.
            let pre_effect_message = self.message.clone();
            let pre_effect_row_open = self.message_row_open_mid_line;
            let pre_effect_revision = self.message_transcript_revision;
            let outcome = self.resolve_dungeon_fall_trap_transition(
                scene,
                level,
                x,
                y,
                Some(game_dir),
                false,
            )?;
            if self.message.is_empty() && self.message_transcript_revision != pre_effect_revision {
                // The off-bottom handoff printed the pit group and then
                // cleared the slot. Its first emit already flushed the
                // command's row, so restoring it would print it a second
                // time; the row state stays as the last emitted line set it.
                return Ok(Some(outcome));
            }
            self.keep_post_turn_effect_rows_separate(&pre_effect_message, pre_effect_row_open);
            return Ok(Some(outcome));
        }
        if is_dungeon_bomb_trap(tile) {
            // `dungeon-mode.md` Section 8: the fired bomb keeps only its
            // visit-marker bit (`0x62` -> `0x00`, `0x6A` -> `0x08`), so the
            // consumed cell cannot fire again (R512).
            self.grid[dungeon_cell_index(level, x, y)] = tile & DUNGEON_VISIT_MARKER_BIT;
            self.mark_visibility_dirty();
            // `dungeon-mode.md` Section 8.1: the two bomb lines, and nothing
            // else. The post-action pass is a producer that can run alongside
            // the command handler, so both lines go straight to the transcript
            // rather than being folded into the compatibility slot.
            self.emit_message_line(DUNGEON_BOMB_TRAP_LINE);
            self.emit_message_line(DUNGEON_KABOOM_LINE);
            // Section 8.1's bomb row also carries "the damage helper's flash
            // and rumble"; see
            // [`PlayState::apply_dungeon_floor_trap_damage`].
            self.apply_dungeon_floor_trap_damage();
            return Ok(None);
        }
        // Section 8.1: only the sleep, poison and fire rows belong to the
        // post-action pass ("Any other underfoot byte: nothing"); electric
        // contact is a movement-time consequence. The field line prints
        // before the per-member rolls.
        if let Some((field, line)) = dungeon_field_effect(tile)
            .and_then(|field| dungeon_field_consequence_line(field).map(|line| (field, line)))
        {
            self.emit_message_line(line);
            let _ = self.apply_dungeon_field_effect_at(level, x, y, tile, field);
            return Ok(None);
        }
        if let Some(outcome) = self.apply_dungeon_active_monster_step(scene, level)? {
            return Ok(Some(outcome));
        }
        Ok(None)
    }

    pub fn apply_dungeon_active_monster_step(
        &mut self,
        scene: DungeonScene,
        level: u8,
    ) -> io::Result<Option<MoveOutcome>> {
        let Some((slot, object)) = self.dungeon_active_monster() else {
            return Ok(None);
        };
        let Some(step) = self.dungeon_active_monster_step(slot, object, level) else {
            return Ok(None);
        };
        if step.contact {
            let direction = self
                .dungeon_direction_from_player_to(step.x, step.y)
                .unwrap_or(self.player.facing);
            // `dungeon-mode.md §14`: the feedback "is assembled from fragments
            // rather than stored whole. The line is `Attacked`, then - only
            // when the computed direction differs from the party's current
            // facing - ` from the ` and one of the lower-case compass words
            // ... and finally `!\n`." It is printed before the new facing is
            // committed, and it is *not* the Doom cave-entrance refusal.
            let line = if direction == self.player.facing {
                DUNGEON_MONSTER_CONTACT_LINE.to_string()
            } else {
                format!(
                    "{DUNGEON_MONSTER_CONTACT_PREFIX} from the {}!\n",
                    direction.name().to_ascii_lowercase()
                )
            };
            self.emit_message_line(line);
            self.player.facing = direction;
            self.free_active_object_slot(slot);
            self.mark_visibility_dirty();
            if combat_class_stats(object.aux1).is_some() {
                let note = self.enter_dungeon_active_monster_combat(level, object)?;
                self.set_dungeon_post_combat_applier(DungeonPostCombatApplier::MovementApplier);
                // The setup report is an engine diagnostic, not a line the
                // original prints; §14's `Attacked` line above is.
                self.push_diagnostic(format!(
                    "Dungeon monster tile {} approaches from the {} on {} level {level}; {note}.",
                    object.tile,
                    direction.name(),
                    scene.key()
                ));
                return Ok(Some(MoveOutcome::Used));
            }
            self.push_diagnostic(format!(
                "Dungeon object tile {} approaches from the {} on {} level {level}; no published combat class.",
                object.tile,
                direction.name(),
                scene.key()
            ));
            return Ok(Some(MoveOutcome::Used));
        }

        // `dungeon-mode.md §6.9`: the dungeon `phase` byte is packed sprite
        // state - symmetry `0x90`, animation mode `0x60` and a four-bit
        // phase - that the painter rebuilds preserving the symmetry and mode
        // bits. A committed step (`§14`) moves the record; it does not write
        // a facing into that byte.
        self.active_objects[slot].x = step.x;
        self.active_objects[slot].y = step.y;
        self.mark_visibility_dirty();
        let move_message = format!(
            "Dungeon monster tile {} moved {} to ({}, {}) on {} level {level}.",
            object.tile,
            step.direction.name(),
            step.x,
            step.y,
            scene.key()
        );
        // A monster taking its step prints nothing: `commands.md §8.1`
        // rules out the tile id, the cell and the scene name this carried,
        // and no section gives an ordinary dungeon monster move a line at
        // all. It was also *appending* to whatever the player's own
        // command had just printed.
        self.push_diagnostic(move_message);
        Ok(None)
    }

    /// `dungeon-mode.md §14` (u5-spec#298 Q33, R551): the active monster's
    /// step is a random walk with no distance test. Up to eight attempts,
    /// each one `random_range(0, 3)` shared-generator draw (0 north, 1 east,
    /// 2 south, 3 west) naming a wrapped neighbour; a candidate of class
    /// `0x6?`, `0x8?` or `0xA?` and up is rejected with no further draw; a
    /// candidate equal to the stored previous cell needs a `random_range(0,
    /// 7)` result of exactly 1. The first acceptance moves current to
    /// previous and the candidate to current. Contact fires only when the
    /// current cell is then the party's cell, and copies the previous cell
    /// back into the current one, so the monster never occupies the party's
    /// cell. The one stationary presentation value is not yet published
    /// (`OPEN-QUESTIONS.md` section 3), so no family is treated as
    /// stationary here.
    pub fn dungeon_active_monster_step(
        &mut self,
        _slot: usize,
        object: ActiveObject,
        level: u8,
    ) -> Option<DungeonActiveMonsterStep> {
        let mut current = (object.x, object.y);
        let mut previous = self.dungeon_monster_previous_cell.unwrap_or(current);
        let mut moved = None;
        for _ in 0..DUNGEON_MONSTER_STEP_ATTEMPTS {
            let direction = dungeon_monster_step_direction(self.random_range_u8(0, 3));
            let (dx, dy) = direction.delta();
            let nx = (current.0 as isize + dx).rem_euclid(DUNGEON_SIDE as isize) as usize;
            let ny = (current.1 as isize + dy).rem_euclid(DUNGEON_SIDE as isize) as usize;
            if !dungeon_active_monster_class_accepts(self.dungeon_cell(level, nx, ny)) {
                continue;
            }
            if (nx, ny) == previous && self.random_range_u8(0, 7) != 1 {
                continue;
            }
            previous = current;
            current = (nx, ny);
            moved = Some(direction);
            break;
        }
        let contact = current == (self.player.x, self.player.y);
        if contact {
            current = previous;
        }
        self.dungeon_monster_previous_cell = Some(previous);
        if moved.is_none() && !contact {
            return None;
        }
        // Without an accepted step the contact rule still applies (Q33:
        // inferred unreachable in normal play); the caller reads no
        // direction on the contact arm.
        Some(DungeonActiveMonsterStep {
            direction: moved.unwrap_or(Direction::North),
            x: current.0,
            y: current.1,
            contact,
        })
    }

    /// The dungeon's single active monster, when its record is live.
    ///
    /// The record's stamped level is **not** consulted. `dungeon-mode.md
    /// §4.1` stamps it on a fresh roll, and every level change, entry and
    /// fight return rolls fresh, so in play it always names the party's
    /// level; the one way in with a different one is a load, which the
    /// setup table makes a *reuse* that "preserves the whole active record",
    /// and §14's step "runs over the current 8-by-8 level" with no level
    /// test. Now published (u5-spec#299 Q3, §4.1 and §6.9's `z` row): "Reuse
    /// validates nothing except activity" - the class byte against `0xFF` -
    /// "The stamped level is never compared with the party's level on reuse,
    /// and no dungeon-mode behaviour depends on it", and the all-zero record
    /// "shows the Giant Rat sprite bank ... and on contact or A-Attack fights
    /// as combat class 0" (family 0 is the rat; combat class 0 is Mages,
    /// R567). Static reading only for a mismatched level
    /// (`OPEN-QUESTIONS.md`). First measured 2026-10-16 (`dungeon-chest-jimmy`, `dungeon-two-way-
    /// ladder`): from saves whose record is all zeros - live, level 0, at
    /// `(0,0)` - the original's monster walked level 3 and attacked "from
    /// the west" in 8 of 22 runs, and this engine, gating on the level,
    /// never did.
    pub fn dungeon_active_monster(&self) -> Option<(usize, ActiveObject)> {
        let Area::Dungeon { .. } = self.area else {
            return None;
        };
        let object = self
            .active_objects
            .get(DUNGEON_ACTIVE_MONSTER_SLOT)
            .copied()?;
        dungeon_monster_record_active(object).then_some((DUNGEON_ACTIVE_MONSTER_SLOT, object))
    }

    pub fn dungeon_direction_from_player_to(&self, x: usize, y: usize) -> Option<Direction> {
        let west = (self.player.x + DUNGEON_SIDE - x) % DUNGEON_SIDE;
        let east = (x + DUNGEON_SIDE - self.player.x) % DUNGEON_SIDE;
        let north = (self.player.y + DUNGEON_SIDE - y) % DUNGEON_SIDE;
        let south = (y + DUNGEON_SIDE - self.player.y) % DUNGEON_SIDE;
        if east == 1 && north == 0 {
            Some(Direction::East)
        } else if west == 1 && north == 0 {
            Some(Direction::West)
        } else if south == 1 && east == 0 {
            Some(Direction::South)
        } else if north == 1 && east == 0 {
            Some(Direction::North)
        } else {
            None
        }
    }

    pub fn hole_up_command(
        &mut self,
        game_dir: &Path,
        request: impl Into<InlineRestRequest>,
    ) -> io::Result<MoveOutcome> {
        self.clear_active_effect_slot();
        let request = request.into();
        match self.area {
            Area::Town { scene, floor } => {
                self.hole_up_town_command(game_dir, request.hours, scene, floor)
            }
            Area::World { .. } | Area::Dungeon { .. } => self.rest_with_watch(request, game_dir),
        }
    }

    pub fn hole_up_town_command(
        &mut self,
        game_dir: &Path,
        hours: Option<u8>,
        scene: Scene,
        floor: i8,
    ) -> io::Result<MoveOutcome> {
        // `commands.md §10`: "The dispatcher first requires the party to be
        // on a bed/inn-style tile. If the tile gate fails, the command is
        // refused" - before any hours prompt. The refusal completes the
        // `Hole up- ` echo line (`cleak/u5-spec#194` capture).
        let entries = load_town_rest_bed_entries(game_dir)?;
        if !self.town_rest_bed_still_accepts(entries.as_deref(), scene, floor) {
            if !self.complete_open_direction_echo("Hole up- ", HOLE_UP_NOT_IN_BED_REFUSAL) {
                self.message = format!("Hole up- {HOLE_UP_NOT_IN_BED_REFUSAL}");
            }
            // `town-mode.md §12` / `rest-and-camp.md §3` (u5-spec#300 Q8,
            // Q14; R588, R603): the off-bed refusal "returns the ordinary
            // acted result, so the refusal costs the normal one-minute town
            // turn and runs the epilogue" (executed).
            self.charge_tile_command_acted_exit(self.turn);
            return Ok(MoveOutcome::Blocked);
        }
        let Some(hours) = hours else {
            return Ok(self.start_rest_prompt());
        };
        if !(1..=9).contains(&hours) {
            self.message.clear();
            return Ok(MoveOutcome::Blocked);
        }

        // `town-mode.md §12` (issue #298 Q30, `RETRACTIONS.md` R538): "the
        // town-hours path runs one burst of up to sixteen schedule/world-tick
        // passes at the current hour, with no clock advance, before anyone
        // sleeps. If any pass leaves an arrest/conflict contact event (kind
        // `0x61`), the command ends at once: no sleep status, no time passed,
        // no further message; it returns the ordinary acted result, so the
        // normal town epilogue follows and re-evaluates contact."
        match self.advance_town_rest_initial_schedule_burst() {
            TownRestBurstOutcome::Completed => {}
            TownRestBurstOutcome::ArrestOrConflictContact => {
                // `npc-schedules.md §9.2` "Event lifetime": the burst "never
                // hands its events to the consumer ... every burst-recorded
                // event is dropped and must be re-raised by a later ordinary
                // pass". The ordinary town epilogue is that pass: one
                // indoor-minute turn whose schedule pass and contact consumer
                // run in the caller's post-turn tail.
                self.pending_town_npc_contact = None;
                self.diagnostics
                    .push("town-bed rest ended by an arrest/conflict contact".to_string());
                self.message.clear();
                self.advance_turn();
                return Ok(MoveOutcome::Passed);
            }
        }
        // `npc-schedules.md §9.2`: every burst-recorded event is dropped,
        // including a conversation contact, which "does not interrupt".
        self.pending_town_npc_contact = None;
        self.mark_town_rest_sleepers();
        if !self.advance_town_rest_until_target_hour(hours, entries.as_deref(), scene, floor) {
            let woke = self.wake_town_rest_sleepers();
            self.diagnostics.push(format!(
                "town-bed rest interrupted; woke {woke} asleep member(s)"
            ));
            // `rest-and-camp.md §4` (u5-spec#300 Q8): the members are put to
            // sleep and only then does `Zzzzzzz...` print, ahead of the
            // ten-minute loop; a failed bed test in a step prints the
            // published refusal `Thrown_out_of_bed!\n` (`commands.md §5.5`).
            self.emit_message_line(format!("{}\n", crate::commands::TOWN_BED_REST_SLEEP_LINE));
            self.message = crate::commands::TOWN_BED_THROWN_OUT_LINE.to_string();
            self.message_row_open_mid_line = false;
            return Ok(MoveOutcome::Blocked);
        }
        let mut recovered_hp = 0;
        let mut recovered_mana = 0;
        for _ in 0..hours {
            let (hp, mana) = self.apply_rest_recovery_tick();
            recovered_hp += hp;
            recovered_mana += mana;
        }
        // `npc-schedules.md §12` (u5-spec#300 Q6, R586): the reseat runs
        // after every ten-minute step inside the loop, not once here.
        let woke = self.wake_town_rest_sleepers();
        // `rest-and-camp.md §4`: the town-bed surface prints its own
        // seven-z sleep line and nothing else. The hours and the recovery
        // totals are engine bookkeeping.
        self.diagnostics.push(format!(
            "town-bed rest {hours}h; recovered {recovered_hp} HP and {recovered_mana} MP; \
             woke {woke} asleep member(s)"
        ));
        self.message = crate::commands::TOWN_BED_REST_SLEEP_LINE.to_string();
        Ok(MoveOutcome::Rested)
    }

    pub fn town_rest_target_hour(current_hour: u8, duration_digit: u8) -> u8 {
        let target = current_hour.saturating_add(duration_digit);
        if target > TOWN_REST_HOUR_WRAP_SUBTRAHEND {
            target - TOWN_REST_HOUR_WRAP_SUBTRAHEND
        } else {
            target
        }
    }

    pub fn advance_town_rest_until_target_hour(
        &mut self,
        duration_digit: u8,
        entries: Option<&[TownRestBedEntry]>,
        scene: Scene,
        floor: i8,
    ) -> bool {
        let target_hour = Self::town_rest_target_hour(self.clock.hour, duration_digit);
        let mut ticks: u16 = 0;
        while self.clock.hour != target_hour && ticks < TOWN_REST_TICK_BUDGET {
            // `town-mode.md §7` (issue #298 Q30, R538): "the elapsed-time
            // loop that follows runs no walker".
            self.advance_turn_with_minutes_policy_and_schedules(
                TOWN_REST_MINUTES_PER_TICK,
                true,
                true,
                true,
                true,
                false,
            );
            // `moons.md §3` (issue #190), the second of the two `H` (Hole
            // up) arms: the census row for "`H` (Hole up), town-bed rest
            // loop" is "On the loop's ten-minute steps", so the repaint is
            // per step rather than once at the end. The strip's cell
            // positions move with the hour and the glyph pair with the day,
            // and a nine-hour rest crosses both.
            self.refresh_cached_moon_glyphs_at_scene_entry();
            ticks += 1;
            // `npc-schedules.md §12` / `rest-and-camp.md §4` (u5-spec#300
            // Q6, R586): the bed loop "reseats the town's NPCs after every
            // ten-minute step, before that step's bed test", after the
            // step's dawn/dusk gate pass (run inside the clock advance
            // above). R586 withdrew "when it finishes its rested hours".
            self.clear_and_replace_scheduled_npcs();
            if !self.town_rest_bed_still_accepts(entries, scene, floor) {
                return false;
            }
        }
        self.clock.hour == target_hour
    }

    /// The town-bed burst of up to sixteen schedule/world-tick passes
    /// (`town-mode.md §12`, R538: "If any pass leaves an arrest/conflict
    /// contact event (kind `0x61`), the command ends at once ... A
    /// conversation-contact event (kind `0x74`) does not interrupt; the next
    /// burst pass clears it ... Otherwise the party sleeps"). The burst does
    /// not re-test the bed tile, which the spec does not describe. Per
    /// `npc-schedules.md §9.2` the burst "never hands its events to the
    /// consumer": the caller drops a returned arrest event too, and the
    /// ordinary town epilogue's own schedule pass re-raises it.
    pub fn advance_town_rest_initial_schedule_burst(&mut self) -> TownRestBurstOutcome {
        for _ in 0..TOWN_REST_INITIAL_SCHEDULE_BURST_TICKS {
            // `karma.md §4.1`: these zero-minute schedule-only setup
            // ticks are not one of the ten-minute rest ticks that age the
            // conversation-payment cooldown. Nor do they run the party
            // status/provision pass: `time.md §12` names exactly four
            // callers, and among rest only "the town-bed rest loop's
            // ten-minute step" (`rest-and-camp.md §5`).
            self.advance_turn_with_minutes_policy(0, true, true, false, false);
            // Issue #298 Q30: the burst "inspects only the arrest/conflict
            // kind `0x61`, and only to abort the rest".
            if self
                .pending_town_npc_contact
                .is_some_and(|event| event.behavior.raises_arrest_or_conflict_contact())
            {
                return TownRestBurstOutcome::ArrestOrConflictContact;
            }
            self.pending_town_npc_contact = None;
        }
        TownRestBurstOutcome::Completed
    }

    pub fn town_rest_bed_still_accepts(
        &self,
        entries: Option<&[TownRestBedEntry]>,
        scene: Scene,
        floor: i8,
    ) -> bool {
        let tile = self.grid[self.player.y * 32 + self.player.x];
        let native_inn_bed =
            crate::shop_session::inn_for_scene(scene.byte).is_some() && is_town_rest_bed_tile(tile);
        native_inn_bed
            || entries.unwrap_or(&[]).iter().any(|entry| {
                town_rest_bed_matches(*entry, scene, floor, self.player.x, self.player.y, tile)
            })
    }
}

/// How the town-bed Hole-up's initial schedule burst ended
/// (`town-mode.md §12`, issue #298 Q30).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TownRestBurstOutcome {
    Completed,
    /// A pass left an arrest/conflict contact event (kind `0x61`).
    ArrestOrConflictContact,
}

pub const fn shadowlord_stonegate_npc_slot(index: usize) -> Option<usize> {
    match index {
        SHADOWLORD_FALSEHOOD_INDEX => Some(SHADOWLORD_FALSEHOOD_STONEGATE_NPC_SLOT),
        SHADOWLORD_HATRED_INDEX => Some(SHADOWLORD_HATRED_STONEGATE_NPC_SLOT),
        SHADOWLORD_COWARDICE_INDEX => Some(SHADOWLORD_COWARDICE_STONEGATE_NPC_SLOT),
        _ => None,
    }
}

/// `combat.md §8` (u5-spec#298 Q1): what a dungeon-arena K resolves on the
/// acting member's own cell before any key is read. See
/// [`PlayState::combat_klimb_current_cell`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatKlimbCurrentCell {
    /// An up ladder: `Up!` at once.
    Up,
    /// A down ladder, or a metal grate in a dungeon-room encounter: `Down!`
    /// at once.
    Down,
    /// The ambush centre built on a two-way ladder: `U/D-`, then a wait for
    /// up or down.
    Both,
}

fn combat_klimb_tile_accepts_vertical(tile: u8, intent: ClimbIntent) -> bool {
    match intent {
        ClimbIntent::Up | ClimbIntent::Down => matches!(tile, 0x50..=0x57),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DungeonActiveMonsterStep {
    pub direction: Direction,
    pub x: usize,
    pub y: usize,
    pub contact: bool,
}

/// `dungeon-mode.md §14` (u5-spec#298 Q33): at most eight step attempts.
pub const DUNGEON_MONSTER_STEP_ATTEMPTS: usize = 8;

/// `dungeon-mode.md §14`: the draw is a cardinal in the party-facing
/// convention of Section 3 - 0 north, 1 east, 2 south, 3 west.
pub fn dungeon_monster_step_direction(draw: u8) -> Direction {
    match draw & 0x03 {
        0 => Direction::North,
        1 => Direction::East,
        2 => Direction::South,
        _ => Direction::West,
    }
}

/// `dungeon-mode.md §14`: reject the pit/trap family `0x6?`, every
/// energy field `0x8?` and anything from `0xA?` up; everything else
/// (`0x9?`, chests, open chests, fountains, ladders, passage) is accepted.
pub fn dungeon_active_monster_class_accepts(tile: u8) -> bool {
    !matches!(tile >> 4, 0x06 | 0x08 | 0x0a..=0x0f)
}

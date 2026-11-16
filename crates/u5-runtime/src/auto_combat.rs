//! Optional auto-combat: the runtime types both shells share (Phase B
//! design 6.3.2; build plan M7-01).
//!
//! Auto-combat is class (b): it acts only through keys a player could type.
//! This module holds the plain settings value [`CombatSettings`] that
//! `resolved.combat()` returns (it never parses anything), the shell-owned
//! bookkeeping [`AutoEngagement`] and [`AutoFightRuntime`], the planned key
//! [`AutoKey`] with the exact byte each shell feeds for it, and the
//! planner's answers [`AutoPlan`], [`HandBack`] and [`AutoPrompt`].
//!
//! Nothing here is stored in [`PlayState`](crate::PlayState), and nothing
//! here reads the transcript. The command-prompt planner
//! [`plan_next_auto_key`] and the observer [`observe_after_auto_key`] are
//! build plan M7-02; the targeting cursor (an auto-owned attempt) is M7-03.

use crate::combat_actor::{
    COMBAT_PARTY_ACTOR_SLOTS, CombatActorDescriptor, CombatArenaFieldKind,
    CombatTargetCandidateView, build_combat_ai_legal_cell_mask, combat_ai_legal_cell_for_actor,
    combat_arena_range, combat_side_census, combat_slot_prompted_by_player_command_handler,
    combat_target_groups_are_hostile, find_combat_ai_target,
};
use crate::combat_arena::COMBAT_ARENA_SIDE;
use crate::combat_targeting::combat_targeting_cursor_step;
use crate::constants::OOL_SLOTS;
use crate::enhancements::idle::IdlePromptKind;
use crate::input_codes::{InputDirection, input_direction_code};
use crate::play_state_struct::PlayState;
use crate::predicates::is_combat_arena_tile_walkable;

/// `combat.step_ms`'s default: the least time between two auto keys.
pub const STEP_MS_DEFAULT: u16 = 150;
/// `combat.step_ms`'s range (6.2.8).
pub const STEP_MS_MIN: u16 = 0;
pub const STEP_MS_MAX: u16 = 2000;
/// `combat.pause_below_hp`'s default, in percent; 0 is off.
pub const PAUSE_BELOW_HP_DEFAULT: u8 = 25;
pub const PAUSE_BELOW_HP_MAX: u8 = 100;
/// `combat.action_cap`'s default and range: auto keys in one fight.
pub const ACTION_CAP_DEFAULT: u16 = 200;
pub const ACTION_CAP_MIN: u16 = 20;
pub const ACTION_CAP_MAX: u16 = 1000;

/// Resolved `[combat]` settings: the typed view `resolved.combat()`
/// returns this. Never parsed here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CombatSettings {
    /// The feature toggle, `combat.auto`.
    pub auto: bool,
    /// Phase 5: prompted allied monsters.
    pub allies: bool,
    /// Phase 4d: shorter cues (deviation tier).
    pub fast_forward: bool,
    pub tactic: AutoTactic,
    pub targeting: AutoTargeting,
    pub interrupt: InterruptMode,
    /// Least milliseconds between two auto keys, `0..=2000`.
    pub step_ms: u16,
    /// Hand a member back below this share of its HP; 0 is off.
    pub pause_below_hp_percent: u8,
    /// Auto keys in one fight before hand-back, `20..=1000`.
    pub action_cap: u16,
    pub victory: VictoryAction,
    pub keep_between_fights: bool,
    pub pause_on_focus_loss: bool,
    /// Phase 4a opt-in.
    pub potions: bool,
    pub mp_reserve: MpReserve,
    /// Spell codes as `catalogs/spell-list.md` section 5 prints them
    /// ("GP"). The engine has no spell-code type yet; the session's
    /// `spell_code` is a `String` too.
    pub spell_allow_list: Vec<String>,
}

impl Default for CombatSettings {
    /// The 6.2.8 defaults; `auto: false` is the faithful game.
    fn default() -> Self {
        Self {
            auto: false,
            allies: false,
            fast_forward: false,
            tactic: AutoTactic::Fight,
            targeting: AutoTargeting::Nearest,
            interrupt: InterruptMode::Swallow,
            step_ms: STEP_MS_DEFAULT,
            pause_below_hp_percent: PAUSE_BELOW_HP_DEFAULT,
            action_cap: ACTION_CAP_DEFAULT,
            victory: VictoryAction::Stop,
            keep_between_fights: false,
            pause_on_focus_loss: true,
            potions: false,
            mp_reserve: MpReserve::OneHeal,
            spell_allow_list: Vec::new(),
        }
    }
}

/// `combat.tactic`. Support and Magic arrive with Phase 4.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AutoTactic {
    #[default]
    Fight,
    Support,
    Magic,
}

/// `combat.targeting`. Weakest arrives with Phase 4.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AutoTargeting {
    #[default]
    Nearest,
    Weakest,
}

/// `combat.interrupt`: what a key the player presses while auto owns the
/// prompt does. Swallow hands back and drops the key; Deliver hands back
/// and passes the key to the game.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InterruptMode {
    #[default]
    Swallow,
    Deliver,
}

/// `combat.victory`. Exit arrives with Phase 4c.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum VictoryAction {
    #[default]
    Stop,
    Exit,
}

/// `combat.mp_reserve` (Phases 4a and 4b): `none`, or `heal`, which keeps
/// enough magic for one healing spell.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MpReserve {
    None,
    #[default]
    OneHeal,
}

/// Which roster positions the player handed over. Shell-owned; never in
/// `PlayState`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AutoEngagement {
    pub members: [bool; 6],
    pub until_done: bool,
}

impl AutoEngagement {
    /// Whether any roster position is handed over.
    pub fn any(&self) -> bool {
        self.members.iter().any(|member| *member)
    }
}

/// A small read-only fingerprint of live state, taken just before and
/// just after an auto key, that recognises "this key changed nothing"
/// (6.3.2). Built from state by the observer (M7-02); never from the
/// transcript.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ProgressPrint {
    pub pending_slot: Option<usize>,
    pub next_slot: usize,
    pub round_counter: u8,
    /// The open targeting cursor's cell, if one is open.
    pub cursor: Option<(u8, u8)>,
    pub prng_state: u16,
    /// A hash of the 32 combat actor rows.
    pub actors_hash: u64,
    /// Each party member's HP, MP and status.
    pub party: [(u16, u8, u8); 6],
}

/// Per-fight counters; reset when `combat_active` goes false.
/// Shell-owned; never in `PlayState`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AutoFightRuntime {
    pub actions: u16,
    pub no_progress: u8,
    /// Cardinal bitmask of steps refused this turn.
    pub blocked_dirs: u8,
    pub turn_slot: Option<usize>,
    pub last_print: Option<ProgressPrint>,
    pub owns_open_attempt: bool,
    pub stopping: Option<HandBack>,
}

/// One planned key. `input_char` is the exact char the per-key dispatcher
/// receives for the same physical key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AutoKey {
    /// `A`.
    Attack,
    /// Space.
    Pass,
    /// A step at the command prompt: cardinals only, `0x01..=0x04`.
    Step(InputDirection),
    /// A targeting-cursor move: `0x01..=0x04` and the diagonals
    /// `0xD3..=0xD6`.
    Cursor(InputDirection),
    /// Return.
    Confirm,
    /// Escape.
    Cancel,
}

impl AutoKey {
    /// The char the per-key dispatcher receives for this key, the same as
    /// for the physical key (6.3.4).
    pub fn input_char(self) -> char {
        match self {
            AutoKey::Attack => 'A',
            AutoKey::Pass => ' ',
            AutoKey::Step(direction) | AutoKey::Cursor(direction) => {
                char::from(input_direction_code(direction))
            }
            AutoKey::Confirm => '\r',
            AutoKey::Cancel => '\u{1b}',
        }
    }

    /// The `--play-script` token that reaches the dispatcher as
    /// [`input_char`](Self::input_char) (6.3.5). Only forms 1.0.1 already
    /// parses: `A`, `pass`, the ANSI navigation sequences, `enter` and a
    /// bare Escape byte.
    pub fn script_token(self) -> &'static str {
        match self {
            AutoKey::Attack => "A",
            AutoKey::Pass => "pass",
            AutoKey::Step(direction) | AutoKey::Cursor(direction) => match direction {
                InputDirection::North => "\x1b[A",
                InputDirection::South => "\x1b[B",
                InputDirection::East => "\x1b[C",
                InputDirection::West => "\x1b[D",
                InputDirection::Northwest => "\x1b[H",
                InputDirection::Northeast => "\x1b[5~",
                InputDirection::Southwest => "\x1b[F",
                InputDirection::Southeast => "\x1b[6~",
            },
            AutoKey::Confirm => "enter",
            AutoKey::Cancel => "\x1b",
        }
    }

    /// Whether the planner may emit this key: a step is cardinal only.
    pub fn is_well_formed(self) -> bool {
        match self {
            AutoKey::Step(direction) => direction.is_cardinal(),
            _ => true,
        }
    }
}

/// Why auto-combat gave a prompt back to the player.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HandBack {
    /// The player pressed a key.
    Interrupted,
    Victory,
    /// The roster position whose HP fell below the limit.
    Hurt(usize),
    NoTarget,
    Stuck,
    ActionCap,
    UnexpectedPrompt,
    FightOver,
}

/// The planner's answer for the current prompt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AutoPlan {
    Key(AutoKey),
    HandBack(HandBack),
    /// Not an auto-owned prompt: leave it to the human.
    NotOurs,
}

/// Which prompt is open, read from state only (6.3.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AutoPrompt {
    None,
    /// The combat command prompt for actor `slot`, roster position
    /// `roster`.
    Command {
        slot: usize,
        /// `None` for a monster-side slot (no roster mapping).
        roster: Option<usize>,
    },
    /// An open targeting cursor for actor `slot`.
    Cursor {
        slot: usize,
    },
    /// A picker, line reader, cast session, ...
    Other,
}

/// The re-prompt budget (6.2.5): after this many no-progress keys in one
/// member's turn the planner passes; a Pass that also makes no progress
/// hands back with [`HandBack::Stuck`]. Fixed, not a setting.
pub const NO_PROGRESS_BUDGET: u8 = 3;

/// Which prompt is open, read from state only (6.3.3).
///
/// `Command` is exactly the foundation's idle combat prompt
/// ([`PlayState::idle_command_prompt`]) with a pending actor slot, so the
/// planner keeps no modal list of its own.
pub fn auto_prompt(state: &PlayState) -> AutoPrompt {
    if !state.combat_active {
        return AutoPrompt::None;
    }
    if let Some(session) = state.active_combat_targeting.as_ref() {
        return AutoPrompt::Cursor {
            slot: session.actor_slot,
        };
    }
    match (state.idle_command_prompt(), state.pending_combat_actor_slot) {
        (Some(IdlePromptKind::Combat), Some(slot)) => AutoPrompt::Command {
            slot,
            roster: state.combat_roster_slot_for_actor_slot(slot),
        },
        _ => AutoPrompt::Other,
    }
}

/// The engaged roster position that owns the command prompt of `slot`, or
/// `None` when the prompt is the human's: a monster-side slot (allies are
/// Phase 5), a member the automatic driver runs (Sword of Chaos, charm),
/// an asleep, dragged-under or dead member, a member solo mode denies, or
/// a roster position not handed over.
fn owned_command_roster(state: &PlayState, slot: usize, engaged: &AutoEngagement) -> Option<usize> {
    let actor = state.combat_actors.get(slot).copied()?;
    let answerable = actor.is_party_side()
        && !actor.is_controlled()
        && !actor.is_status_disabled()
        && !actor.is_dragged_under()
        && combat_slot_prompted_by_player_command_handler(state.active_player, slot, actor);
    if !answerable {
        return None;
    }
    let roster = state.combat_roster_slot_for_actor_slot(slot)?;
    engaged.members.get(roster).copied()?.then_some(roster)
}

/// Pure: reads live state, draws nothing, mutates nothing (6.3.2).
///
/// A pending [`AutoFightRuntime::stopping`] never cuts an auto-owned
/// targeting attempt short (6.2.4): the cursor keeps being planned until
/// the `A` walk closes, and the hand-back is reported at the next prompt.
pub fn plan_next_auto_key(
    state: &PlayState,
    settings: &CombatSettings,
    engaged: &AutoEngagement,
    fight: &AutoFightRuntime,
) -> AutoPlan {
    let prompt = auto_prompt(state);
    if let AutoPrompt::Cursor { slot } = prompt
        && fight.owns_open_attempt
    {
        return plan_cursor(state, fight, slot);
    }
    if let Some(reason) = fight.stopping {
        return AutoPlan::HandBack(reason);
    }
    match prompt {
        AutoPrompt::Command { slot, .. } => match owned_command_roster(state, slot, engaged) {
            Some(_) => plan_command_prompt(state, settings, fight, slot),
            None => AutoPlan::NotOurs,
        },
        // A cursor the player opened is the player's.
        AutoPrompt::Cursor { .. } | AutoPrompt::None | AutoPrompt::Other => AutoPlan::NotOurs,
    }
}

/// The eight cursor moves in the order the search tries them: the
/// cardinals, then the diagonals (`systems/combat.md` 8.2).
const CURSOR_MOVES: [InputDirection; 8] = [
    InputDirection::West,
    InputDirection::East,
    InputDirection::North,
    InputDirection::South,
    InputDirection::Northwest,
    InputDirection::Northeast,
    InputDirection::Southwest,
    InputDirection::Southeast,
];

/// An auto-owned targeting attempt (6.3.3): re-pick the target among the
/// eligible occupants within this attempt's `max_range`, Confirm only on
/// that occupant's cell, Cancel when nothing is in reach, otherwise one
/// cursor move along a shortest path. Never Space; never Enter on the
/// attacker's own cell.
fn plan_cursor(state: &PlayState, fight: &AutoFightRuntime, slot: usize) -> AutoPlan {
    let Some(session) = state.active_combat_targeting.as_ref() else {
        return AutoPlan::HandBack(HandBack::UnexpectedPrompt);
    };
    let side = COMBAT_ARENA_SIDE as u8;
    let in_arena = |cell: (u8, u8)| cell.0 < side && cell.1 < side;
    // An attempt auto did not open, or a session the engine never builds.
    if fight.turn_slot != Some(slot)
        || session.attempt().is_none()
        || !in_arena(session.attacker)
        || !in_arena(session.cursor)
        || combat_arena_range(
            session.attacker.0,
            session.attacker.1,
            session.cursor.0,
            session.cursor.1,
        ) > session.max_range
    {
        return AutoPlan::HandBack(HandBack::UnexpectedPrompt);
    }
    // Cursor keys that keep changing nothing: give the attempt up, which
    // spends it exactly as a human's Escape does.
    if fight.no_progress >= NO_PROGRESS_BUDGET {
        return AutoPlan::Key(AutoKey::Cancel);
    }
    let Some(target) = cursor_target(state, slot, session.attacker, session.max_range) else {
        return AutoPlan::Key(AutoKey::Cancel);
    };
    if session.cursor == target {
        return AutoPlan::Key(AutoKey::Confirm);
    }
    match cursor_path_first_move(session.cursor, target, session.attacker, session.max_range) {
        Some(direction) => AutoPlan::Key(AutoKey::Cursor(direction)),
        None => AutoPlan::Key(AutoKey::Cancel),
    }
}

/// The cell of the target this attempt should strike: the nearest eligible
/// hostile (the command prompt's rule, suppression filter on) among those
/// within `max_range` of the attacker whose cell
/// `combat_targeting_occupant_at` resolves to that very slot, never the
/// attacker's own cell.
fn cursor_target(
    state: &PlayState,
    slot: usize,
    attacker: (u8, u8),
    max_range: u8,
) -> Option<(u8, u8)> {
    let acting_group = state.combat_target_group_for_slot(slot);
    let mut views = auto_candidate_views(state, acting_group);
    for (candidate, view) in views.iter_mut().enumerate() {
        let reachable = state.combat_actor_cell(candidate).is_some_and(|cell| {
            cell != attacker
                && combat_arena_range(attacker.0, attacker.1, cell.0, cell.1) <= max_range
                && state.combat_targeting_occupant_at(cell) == Some(candidate)
        });
        view.suppressed = view.suppressed || !reachable;
    }
    let target = find_combat_ai_target(&views, slot, acting_group, false).slot?;
    state.combat_actor_cell(target)
}

/// A breadth-first search over cursor cells from `cursor` to `target`,
/// with the engine's own `combat_targeting_cursor_step` as the move
/// function, so the 11x11 bounds and the range clamp are exactly the
/// engine's. Returns the first move of a shortest path.
fn cursor_path_first_move(
    cursor: (u8, u8),
    target: (u8, u8),
    attacker: (u8, u8),
    max_range: u8,
) -> Option<InputDirection> {
    let index = |cell: (u8, u8)| usize::from(cell.1) * COMBAT_ARENA_SIDE + usize::from(cell.0);
    let mut first: [Option<InputDirection>; COMBAT_ARENA_SIDE * COMBAT_ARENA_SIDE] =
        [None; COMBAT_ARENA_SIDE * COMBAT_ARENA_SIDE];
    let mut seen = [false; COMBAT_ARENA_SIDE * COMBAT_ARENA_SIDE];
    let mut queue = std::collections::VecDeque::new();
    seen[index(cursor)] = true;
    queue.push_back(cursor);
    while let Some(cell) = queue.pop_front() {
        for direction in CURSOR_MOVES {
            let Some(next) = combat_targeting_cursor_step(cell, attacker, max_range, direction)
            else {
                continue;
            };
            if seen[index(next)] {
                continue;
            }
            seen[index(next)] = true;
            first[index(next)] = first[index(cell)].or(Some(direction));
            if next == target {
                return first[index(next)];
            }
            queue.push_back(next);
        }
    }
    None
}

/// The Fight tactic at an owned command prompt (6.2.3, 6.3.3).
fn plan_command_prompt(
    state: &PlayState,
    settings: &CombatSettings,
    fight: &AutoFightRuntime,
    slot: usize,
) -> AutoPlan {
    // 1. Stop checks, in order: victory, cap, HP limit, budget.
    if !combat_side_census(&state.combat_actors).foes_remain() {
        return AutoPlan::HandBack(HandBack::Victory);
    }
    if fight.actions >= settings.action_cap {
        return AutoPlan::HandBack(HandBack::ActionCap);
    }
    if let Some(roster) = hurt_member(state, settings.pause_below_hp_percent) {
        return AutoPlan::HandBack(HandBack::Hurt(roster));
    }
    let (no_progress, blocked_dirs) = if fight.turn_slot == Some(slot) {
        (fight.no_progress, fight.blocked_dirs)
    } else {
        (0, 0)
    };
    if no_progress > NO_PROGRESS_BUDGET {
        return AutoPlan::HandBack(HandBack::Stuck);
    }
    if no_progress >= NO_PROGRESS_BUDGET {
        return AutoPlan::Key(AutoKey::Pass);
    }

    // 2. The nearest visible hostile that is not a roster member.
    let Some(target) = auto_target(state, slot) else {
        return AutoPlan::HandBack(HandBack::NoTarget);
    };
    let (Some(from), Some(to)) = (
        state.combat_actor_cell(slot),
        state.combat_actor_cell(target),
    ) else {
        return AutoPlan::Key(AutoKey::Pass);
    };

    // 3. Attack when a readied attempt reaches and is not interfered.
    let range = combat_arena_range(from.0, from.1, to.0, to.1);
    let interfered = state
        .combat_attack_interference_source_for_slot(slot)
        .is_some();
    let attempts = state.combat_attack_attempts_for_actor(slot);
    let mut reaching = attempts
        .iter()
        .filter(|attempt| attempt.max_range >= range)
        .peekable();
    if reaching.peek().is_some() {
        if reaching.any(|attempt| !(attempt.runs_interference && interfered)) {
            return AutoPlan::Key(AutoKey::Attack);
        }
        // Every reaching attempt would be aborted: the same spent turn,
        // and a cleaner transcript.
        return AutoPlan::Key(AutoKey::Pass);
    }

    // 4. A cardinal step toward the target, 5. otherwise Pass.
    match auto_step(state, from, to, blocked_dirs) {
        Some(direction) => AutoPlan::Key(AutoKey::Step(direction)),
        None => AutoPlan::Key(AutoKey::Pass),
    }
}

/// The first roster position whose living member is below the HP limit.
fn hurt_member(state: &PlayState, percent: u8) -> Option<usize> {
    if percent == 0 {
        return None;
    }
    state
        .party
        .iter()
        .take(COMBAT_PARTY_ACTOR_SLOTS)
        .position(|member| {
            member.living()
                && u32::from(member.hp) * 100 < u32::from(member.max_hp) * u32::from(percent)
        })
}

/// Whether the arena draws this actor: the suppression filter (0x10) and
/// the hidden-frame marker of its linked record (no spoilers, 6.2.3).
fn actor_visible(state: &PlayState, actor: CombatActorDescriptor) -> bool {
    !actor.is_phase_suppressed() && state.combat_actor_presentation_displayed(actor)
}

/// The planner's own 32-row copy of the candidate views, as the automatic
/// driver builds them, with every row the planner must never pick marked
/// suppressed rather than removed (`find_combat_ai_target` indexes the
/// slice by slot): invisible actors, roster and party-side rows, and rows
/// not hostile to the acting group.
fn auto_candidate_views(state: &PlayState, acting_group: u8) -> Vec<CombatTargetCandidateView> {
    state
        .combat_actors
        .iter()
        .copied()
        .enumerate()
        .map(|(slot, descriptor)| {
            let mut view = state.combat_target_candidate_view(
                descriptor,
                slot,
                descriptor.is_phase_suppressed(),
                descriptor.is_dragged_under(),
            );
            view.suppressed = view.suppressed
                || !actor_visible(state, descriptor)
                || descriptor.is_party_side()
                || state.combat_roster_slot_for_actor_slot(slot).is_some()
                || !combat_target_groups_are_hostile(acting_group, view.group);
            view
        })
        .collect()
}

/// The target `slot` would attack: `find_combat_ai_target` over the
/// planner's views, suppression filter on.
fn auto_target(state: &PlayState, slot: usize) -> Option<usize> {
    let acting_group = state.combat_target_group_for_slot(slot);
    let views = auto_candidate_views(state, acting_group);
    find_combat_ai_target(&views, slot, acting_group, false).slot
}

const CARDINALS: [InputDirection; 4] = [
    InputDirection::West,
    InputDirection::East,
    InputDirection::North,
    InputDirection::South,
];

/// The [`AutoFightRuntime::blocked_dirs`] bit of a cardinal direction.
pub fn blocked_dir_bit(direction: InputDirection) -> u8 {
    CARDINALS
        .iter()
        .position(|cardinal| *cardinal == direction)
        .map_or(0, |index| 1 << index)
}

fn cardinal_delta(direction: InputDirection) -> (i16, i16) {
    match direction {
        InputDirection::West => (-1, 0),
        InputDirection::East => (1, 0),
        InputDirection::North => (0, -1),
        _ => (0, 1),
    }
}

/// Whether a live combat field marker lies on the cell.
fn field_marker_at(state: &PlayState, x: usize, y: usize) -> bool {
    let placed = state
        .combat_magic_effects
        .get(y)
        .and_then(|row| row.get(x))
        .is_some_and(|effect| *effect != 0);
    placed
        || state.active_objects.iter().take(OOL_SLOTS).any(|object| {
            object.x == x
                && object.y == y
                && CombatArenaFieldKind::from_kind_byte(object.type_byte).is_some()
        })
}

/// One cardinal step from `from` toward `to`: the larger-delta axis first,
/// then the other. The legal-cell mask is only a hint, built with every
/// invisible actor left out so the plan never avoids an unseen monster;
/// out of bounds is never legal (`fleeing = false`). Field markers and
/// this turn's refused directions are skipped.
fn auto_step(
    state: &PlayState,
    from: (u8, u8),
    to: (u8, u8),
    blocked_dirs: u8,
) -> Option<InputDirection> {
    let seen_actors = state.combat_actors.map(|actor| {
        if actor_visible(state, actor) {
            actor
        } else {
            CombatActorDescriptor::empty()
        }
    });
    let mask = build_combat_ai_legal_cell_mask(
        &state.combat_terrain,
        &seen_actors,
        is_combat_arena_tile_walkable,
    );
    let dx = i16::from(to.0) - i16::from(from.0);
    let dy = i16::from(to.1) - i16::from(from.1);
    let horizontal = match dx.signum() {
        1 => Some(InputDirection::East),
        -1 => Some(InputDirection::West),
        _ => None,
    };
    let vertical = match dy.signum() {
        1 => Some(InputDirection::South),
        -1 => Some(InputDirection::North),
        _ => None,
    };
    let order = if dx.abs() >= dy.abs() {
        [horizontal, vertical]
    } else {
        [vertical, horizontal]
    };
    order.into_iter().flatten().find(|direction| {
        let (step_x, step_y) = cardinal_delta(*direction);
        let (x, y) = (i16::from(from.0) + step_x, i16::from(from.1) + step_y);
        blocked_dirs & blocked_dir_bit(*direction) == 0
            && combat_ai_legal_cell_for_actor(&mask, x, y, false)
            && !field_marker_at(state, x as usize, y as usize)
    })
}

/// The read-only fingerprint of live state (6.3.2). Never reads the
/// transcript.
pub fn progress_print(state: &PlayState) -> ProgressPrint {
    // FNV-1a over the 32 raw rows: stable across runs and platforms.
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for actor in state.combat_actors.iter() {
        for byte in actor.raw_row() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    let mut party = [(0, 0, 0); 6];
    for (entry, member) in party.iter_mut().zip(state.party.iter()) {
        *entry = (member.hp, member.mana, member.status);
    }
    ProgressPrint {
        pending_slot: state.pending_combat_actor_slot,
        next_slot: state.next_combat_actor_slot,
        round_counter: state.combat_round_counter,
        cursor: state
            .active_combat_targeting
            .as_ref()
            .map(|session| session.cursor),
        prng_state: state.prng_state,
        actors_hash: hash,
        party,
    }
}

/// Called by the shell immediately before it applies an auto key, with no
/// idle pass in between: takes the "before" print.
pub fn record_print_before_auto_key(state: &PlayState, fight: &mut AutoFightRuntime) {
    fight.last_print = Some(progress_print(state));
}

/// Called after the shell applied `key` (6.3.2): classifies it as progress
/// or no progress against the print taken just before, updates the
/// counters, and records whether an auto `A` owns the open cursor.
pub fn observe_after_auto_key(state: &PlayState, key: AutoKey, fight: &mut AutoFightRuntime) {
    let after = progress_print(state);
    let progressed = fight.last_print.is_none_or(|before| before != after);
    fight.actions = fight.actions.saturating_add(1);
    if progressed {
        fight.no_progress = 0;
        fight.blocked_dirs = 0;
    } else {
        match key {
            AutoKey::Step(direction) => {
                fight.blocked_dirs |= blocked_dir_bit(direction);
                fight.no_progress = fight.no_progress.saturating_add(1);
            }
            // The fallback itself changed nothing: next plan hands back.
            AutoKey::Pass => fight.no_progress = NO_PROGRESS_BUDGET + 1,
            _ => fight.no_progress = fight.no_progress.saturating_add(1),
        }
    }
    let cursor_slot = state
        .active_combat_targeting
        .as_ref()
        .map(|session| session.actor_slot);
    fight.owns_open_attempt = cursor_slot.is_some()
        && matches!(
            key,
            AutoKey::Attack | AutoKey::Cursor(_) | AutoKey::Confirm | AutoKey::Cancel
        );
    fight.turn_slot = cursor_slot.or(state.pending_combat_actor_slot);
    fight.last_print = Some(after);
}

#[cfg(test)]
mod cursor_tests;
#[cfg(test)]
mod planner_tests;
#[cfg(test)]
mod tests;

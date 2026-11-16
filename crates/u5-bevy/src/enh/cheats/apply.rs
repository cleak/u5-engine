//! The idle-prompt application point (Phase B design 7.3.4, 7.3.5).
//!
//! `cheat_apply` runs in `EnhSet::Observe`, after the input drivers, only
//! while `cheats.enabled` is on, a state-altering cheat is effective, a
//! [`CheatSession`] and the `VisualState` exist, and the game waits at an
//! exploration or combat idle prompt (both halves of the foundation's
//! predicate). By then the frame's typed keys are drained, so nothing can
//! start a turn between the page's key and its effect.
//!
//! Each frame it runs it:
//!
//! 1. drains the page's queue in order, checking each action's own key
//!    again (a cheat switched off at runtime stops at once while the
//!    others stay on);
//! 2. applies the engaged locks, masked by the lock permissions in force,
//!    once per lock epoch `(turn, combat_active, combat_round_counter)`:
//!    idle frames at the same prompt never apply them again;
//! 3. when anything changed, taints the session, sets the `MOD` badge and
//!    repaints the frame with the faithful renderer, except in the
//!    first-person dungeon, where a repaint could draw the corridor
//!    painter's randomness: there the next natural redraw shows it
//!    (7.3.5).

use bevy::prelude::*;
use u5_runtime::cheats::{
    CheatEdit, CheatRefusal, EditReport, EngagedLocks, TeleportPlan, TeleportTarget,
    apply_cheat_edit, apply_cheat_edits, apply_locks, commit_teleport, plan_teleport,
};
use u5_runtime::enhancements::{CheatToggles, LockPermissions};
use u5_runtime::{Area, PlayState, REAGENT_COUNT, TransportState};

use super::page::CheatPage;
use super::teleport::{LOAD_FAILED, TELEPORTED, refusal_text};
use super::{CheatSession, Toggles, cheat_toggles};
use crate::VisualState;
use crate::enh::toast::ShellBadges;

/// Result-line words (engine text, 7.2.3).
const HEALED: &str = "Healed";
const CURED: &str = "Cured";
const RAISED: &str = "Raised";
const MEMBER: &str = "member";
const MEMBERS: &str = "members";
const HULL: &str = "Hull";
const HULL_REPAIRED: &str = "Hull repaired.";
const GOLD: &str = "Gold";
const FOOD: &str = "Food";
const ARROWS: &str = "Arrows";
const QUARRELS: &str = "Quarrels";
const REAGENTS: &str = "Reagents";
const LOCK_ON: &str = "lock on.";
const LOCK_OFF: &str = "lock off.";
const CHEAT_OFF: &str = "That cheat is off.";
/// Why a repaint is made, for the shell's frame log.
const REPAINT_REASON: &str = "cheats: repaint after a change";

/// A quick action (7.2.4), in the page's order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum QuickAction {
    HealAll,
    CurePoison,
    ResurrectAll,
    MaxGold,
    MaxFood,
    RefillArrows,
    MaxReagents,
    RepairHull,
}

impl QuickAction {
    pub const ALL: [QuickAction; 8] = [
        QuickAction::HealAll,
        QuickAction::CurePoison,
        QuickAction::ResurrectAll,
        QuickAction::MaxGold,
        QuickAction::MaxFood,
        QuickAction::RefillArrows,
        QuickAction::MaxReagents,
        QuickAction::RepairHull,
    ];

    /// Whether its key is effectively on.
    pub fn allowed(self, cheats: &CheatToggles) -> bool {
        cheats.allows(match self {
            QuickAction::HealAll => cheats.heal_all,
            QuickAction::CurePoison => cheats.cure_poison,
            QuickAction::ResurrectAll => cheats.resurrect_all,
            QuickAction::MaxGold => cheats.max_gold,
            QuickAction::MaxFood => cheats.max_food,
            QuickAction::RefillArrows => cheats.refill_arrows,
            QuickAction::MaxReagents => cheats.max_reagents,
            QuickAction::RepairHull => cheats.repair_hull,
        })
    }

    /// The runtime edit.
    pub fn edit(self, cheats: &CheatToggles) -> CheatEdit {
        match self {
            QuickAction::HealAll => CheatEdit::HealAll,
            QuickAction::CurePoison => CheatEdit::CurePoison,
            QuickAction::ResurrectAll => CheatEdit::ResurrectAll,
            QuickAction::MaxGold => CheatEdit::MaxGold,
            QuickAction::MaxFood => CheatEdit::MaxFood,
            QuickAction::RefillArrows => CheatEdit::RefillArrows {
                quarrels: cheats.refill_quarrels,
            },
            QuickAction::MaxReagents => CheatEdit::MaxReagents,
            QuickAction::RepairHull => CheatEdit::RepairHull { parked: true },
        }
    }

    /// Whether it cannot work now, so the page draws it dimmed (7.2.3).
    pub fn impossible_now(self, state: &PlayState) -> bool {
        match self {
            QuickAction::CurePoison | QuickAction::ResurrectAll => state.combat_active,
            QuickAction::RepairHull => {
                !matches!(state.player.transport, TransportState::Ship { .. })
                    && (state.combat_active || !matches!(state.area, Area::World { .. }))
            }
            _ => false,
        }
    }
}

/// A lock (7.2.5), in the page's order: its digit is its index plus one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LockKind {
    Health,
    Mana,
    Food,
    Gold,
    Keys,
    Gems,
    Torches,
    Arrows,
    Reagents,
}

impl LockKind {
    pub const COUNT: usize = 9;
    pub const ALL: [LockKind; LockKind::COUNT] = [
        LockKind::Health,
        LockKind::Mana,
        LockKind::Food,
        LockKind::Gold,
        LockKind::Keys,
        LockKind::Gems,
        LockKind::Torches,
        LockKind::Arrows,
        LockKind::Reagents,
    ];

    /// Whether its key is effectively on.
    pub fn permitted(self, locks: &LockPermissions) -> bool {
        match self {
            LockKind::Health => locks.health,
            LockKind::Mana => locks.mana,
            LockKind::Food => locks.food,
            LockKind::Gold => locks.gold,
            LockKind::Keys => locks.keys,
            LockKind::Gems => locks.gems,
            LockKind::Torches => locks.torches,
            LockKind::Arrows => locks.arrows,
            LockKind::Reagents => locks.reagents,
        }
    }

    /// The page's label (engine text).
    pub const fn label(self) -> &'static str {
        match self {
            LockKind::Health => "Health",
            LockKind::Mana => "Mana",
            LockKind::Food => "Food",
            LockKind::Gold => "Gold",
            LockKind::Keys => "Keys",
            LockKind::Gems => "Gems",
            LockKind::Torches => "Torches",
            LockKind::Arrows => "Arrows",
            LockKind::Reagents => "Reagents",
        }
    }
}

/// What the page asked for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CheatAction {
    Quick(QuickAction),
    /// A lock was checked: it applies once at once, without moving the
    /// epoch (7.3.4).
    LockNow(LockKind),
    /// Edit party and stores' Apply: every changed field, applied as one
    /// change (7.2.6). An empty list is the empty Apply: `NothingToDo`,
    /// and the session stays untainted.
    Edit(Vec<CheatEdit>),
    /// Teleport's Enter (7.2.7): planned again here, then committed.
    Teleport(TeleportTarget),
}

/// The lock epoch: one per turn and one per combat round (7.3.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LockEpoch {
    pub turn: u64,
    pub combat_active: bool,
    pub combat_round_counter: u8,
}

impl LockEpoch {
    pub fn of(state: &PlayState) -> Self {
        Self {
            turn: state.turn,
            combat_active: state.combat_active,
            combat_round_counter: state.combat_round_counter,
        }
    }
}

/// The engaged locks the runtime applies: the checked ones, masked by the
/// permissions in force, with the quarrels following the arrows when
/// `refill_quarrels` is on.
pub fn engaged_locks(engaged: &[bool; LockKind::COUNT], cheats: &CheatToggles) -> EngagedLocks {
    let on = |lock: LockKind| engaged[lock as usize] && lock.permitted(&cheats.locks);
    EngagedLocks {
        health: on(LockKind::Health),
        mana: on(LockKind::Mana),
        food: on(LockKind::Food),
        gold: on(LockKind::Gold),
        keys: on(LockKind::Keys),
        gems: on(LockKind::Gems),
        torches: on(LockKind::Torches),
        arrows: on(LockKind::Arrows),
        quarrels: on(LockKind::Arrows) && cheats.refill_quarrels,
        reagents: [on(LockKind::Reagents); REAGENT_COUNT],
    }
}

/// Run condition: a state-altering cheat is effective.
pub(super) fn state_altering_on(toggles: Option<Res<Toggles>>) -> bool {
    toggles.is_some_and(|toggles| cheat_toggles(toggles.0).any_state_altering())
}

/// "Healed 2 members."
fn counted(verb: &str, count: u8) -> String {
    let noun = if count == 1 { MEMBER } else { MEMBERS };
    format!("{verb} {count} {noun}.")
}

/// The result line for an applied quick action.
fn quick_result(action: QuickAction, report: &EditReport, state: &PlayState) -> String {
    let hull = match state.player.transport {
        TransportState::Ship { hull, .. } => Some(hull),
        _ => None,
    };
    match action {
        QuickAction::HealAll => {
            let mut line = if report.members_touched > 0 {
                counted(HEALED, report.members_touched)
            } else {
                String::new()
            };
            if let Some(hull) = hull {
                if !line.is_empty() {
                    line.push(' ');
                }
                line.push_str(&format!("{HULL} {hull}."));
            }
            line
        }
        QuickAction::CurePoison => counted(CURED, report.members_touched),
        QuickAction::ResurrectAll => counted(RAISED, report.members_touched),
        QuickAction::MaxGold => format!("{GOLD} {}.", state.gold),
        QuickAction::MaxFood => format!("{FOOD} {}.", state.food),
        QuickAction::RefillArrows => format!(
            "{ARROWS} {}. {QUARRELS} {}.",
            state.equipment_stock[u5_runtime::EQUIPMENT_ID_ARROWS],
            state.equipment_stock[u5_runtime::EQUIPMENT_ID_QUARRELS],
        ),
        QuickAction::MaxReagents => format!(
            "{REAGENTS} {}.",
            state.reagents.iter().min().copied().unwrap_or_default()
        ),
        QuickAction::RepairHull => match hull {
            Some(hull) => format!("{HULL} {hull}."),
            None => HULL_REPAIRED.to_string(),
        },
    }
}

/// The result line for a refusal.
fn refusal_result(refusal: CheatRefusal) -> String {
    format!("{}.", refusal.message())
}

/// Applies the queue and the locks (see the module notes).
pub(super) fn cheat_apply(
    toggles: Res<Toggles>,
    mut session: ResMut<CheatSession>,
    mut visual: ResMut<VisualState>,
    mut images: ResMut<Assets<Image>>,
    page: Option<ResMut<CheatPage>>,
    badges: Option<ResMut<ShellBadges>>,
) {
    let cheats = cheat_toggles(toggles.0);
    let session = &mut *session;
    session.repaint_deferred = false;
    let epoch = LockEpoch::of(&visual.state);
    let locks = engaged_locks(&session.engaged, &cheats);
    let epoch_due = locks.any() && session.last_epoch != Some(epoch);
    if session.queued.is_empty() && !epoch_due {
        session.last_epoch = Some(epoch);
        return;
    }
    // Only now is the state borrowed for writing, so an idle frame with
    // nothing to do leaves `VisualState` unchanged.
    let game_dir = visual.game_dir.clone();
    let state = &mut visual.state;
    let mut changed = false;
    let mut result = None;
    let mut edit_applied = false;
    let mut teleported = false;
    for action in std::mem::take(&mut session.queued) {
        match action {
            CheatAction::Quick(quick) => {
                if !quick.allowed(&cheats) {
                    result = Some(CHEAT_OFF.to_string());
                    continue;
                }
                match apply_cheat_edit(state, &quick.edit(&cheats)) {
                    Ok(report) => {
                        changed |= report.changed;
                        result = Some(quick_result(quick, &report, state));
                    }
                    Err(refusal) => result = Some(refusal_result(refusal)),
                }
            }
            CheatAction::LockNow(lock) => {
                if !lock.permitted(&cheats.locks) || !session.engaged[lock as usize] {
                    continue;
                }
                let mut only = [false; LockKind::COUNT];
                only[lock as usize] = true;
                changed |= apply_locks(state, &engaged_locks(&only, &cheats));
                result = Some(format!("{} {LOCK_ON}", lock.label()));
            }
            CheatAction::Edit(edits) => {
                if !cheats.allows(cheats.stat_edit) {
                    result = Some(CHEAT_OFF.to_string());
                    continue;
                }
                match apply_cheat_edits(state, &edits) {
                    Ok(report) => {
                        changed |= report.changed;
                        edit_applied = true;
                        result = Some(super::edit::applied_result());
                    }
                    Err(refusal) => result = Some(refusal_result(refusal)),
                }
            }
            CheatAction::Teleport(target) => {
                if !cheats.allows(cheats.teleport) {
                    result = Some(CHEAT_OFF.to_string());
                    continue;
                }
                result = Some(teleport(state, &game_dir, target).to_string());
                teleported = true;
                changed |= result.as_deref() == Some(TELEPORTED);
            }
        }
    }
    if epoch_due {
        changed |= apply_locks(state, &locks);
    }
    session.last_epoch = Some(epoch);
    if let Some(mut page) = page {
        if edit_applied {
            page.refresh_edit(state);
        }
        if teleported {
            page.refresh_teleport(state, &game_dir);
        }
        if let Some(text) = result {
            page.set_result(text);
        }
    }
    if !changed {
        return;
    }
    session.tainted = true;
    if let Some(mut badges) = badges {
        badges.modified = true;
    }
    repaint(&mut visual, &mut images, session);
}

/// Plans `target` again at the prompt and commits it: the result line.
/// A refusal or a load failure changes nothing (7.2.10).
fn teleport(
    state: &mut PlayState,
    game_dir: &std::path::Path,
    target: TeleportTarget,
) -> &'static str {
    match plan_teleport(state, game_dir, target) {
        plan @ TeleportPlan::Ready { .. } => match commit_teleport(state, game_dir, &plan) {
            Ok(()) => TELEPORTED,
            Err(_) => LOAD_FAILED,
        },
        TeleportPlan::Refused(refusal) => refusal_text(&refusal),
    }
}

/// The result line when a lock is unchecked.
pub(super) fn lock_off_result(lock: LockKind) -> String {
    format!("{} {LOCK_OFF}", lock.label())
}

/// Whether a repaint here could draw randomness: the first-person dungeon
/// view (7.3.5). Combat in a dungeon is drawn top-down.
pub(crate) fn repaint_must_wait(state: &PlayState) -> bool {
    matches!(state.area, Area::Dungeon { .. }) && !state.combat_active
}

/// Repaints the frame after a change, or leaves it to the next natural
/// redraw in the first-person dungeon (7.3.5).
fn repaint(visual: &mut VisualState, images: &mut Assets<Image>, session: &mut CheatSession) {
    if repaint_must_wait(&visual.state) {
        session.repaint_deferred = true;
        return;
    }
    crate::repaint_visual_frame(visual, images, REPAINT_REASON);
}

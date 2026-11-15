//! The `[cheats]` settings as the engine sees them (Phase B design 7.3.2;
//! build plan M8-01).
//!
//! The rows are in the catalogue ([`super::catalogue`]): the section switch
//! `cheats.enabled`, one class (c) row per cheat, and the class (a) watch.
//! This module holds the typed view [`ResolvedEnhancements::cheats`], which
//! turns the resolved bitset into the plain [`CheatToggles`] the cheats
//! plugin reads. It is plain data: no serde, no Bevy, no paths.
//!
//! Every class (c) entry is masked by `enabled` here as well as in the
//! resolver, so a hand-built [`ResolvedEnhancements`] cannot open a cheat
//! without the switch. Under the parity lock the view is
//! [`CheatToggles::default`], every field false (7.4.5).

use super::catalogue::Toggle;
use super::resolve::ResolvedEnhancements;
use super::source::LockState;

/// Which locks the player may check on the Cheats page (7.2.5, 7.3.2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LockPermissions {
    pub health: bool,
    pub mana: bool,
    pub food: bool,
    pub gold: bool,
    pub keys: bool,
    pub gems: bool,
    pub torches: bool,
    pub arrows: bool,
    pub reagents: bool,
}

impl LockPermissions {
    /// Whether any lock may be checked.
    pub fn any(&self) -> bool {
        self.health
            || self.mana
            || self.food
            || self.gold
            || self.keys
            || self.gems
            || self.torches
            || self.arrows
            || self.reagents
    }
}

/// The typed `[cheats]` view (7.3.2). The default is every key off.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CheatToggles {
    /// Section safety switch. Gates every class (c) entry.
    pub enabled: bool,
    pub heal_all: bool,
    pub cure_poison: bool,
    pub resurrect_all: bool,
    pub max_gold: bool,
    pub max_food: bool,
    pub refill_arrows: bool,
    /// Sub-option of `refill_arrows` and the arrow lock; enables nothing
    /// alone.
    pub refill_quarrels: bool,
    pub max_reagents: bool,
    pub repair_hull: bool,
    pub locks: LockPermissions,
    /// Read at launch only; enables nothing alone.
    pub remember_locks: bool,
    pub stat_edit: bool,
    pub teleport: bool,
    /// Class (a). Not gated by `enabled` (D7.10).
    pub state_watch: bool,
}

impl CheatToggles {
    /// Effective permission for a class (c) entry.
    pub fn allows(&self, entry: bool) -> bool {
        self.enabled && entry
    }

    /// Whether any cheat that changes the game is effectively on.
    /// `refill_quarrels` and `remember_locks` are left out: on their own
    /// they enable nothing, so they cannot make the plugin load (7.3.2).
    pub fn any_state_altering(&self) -> bool {
        self.enabled
            && (self.heal_all
                || self.cure_poison
                || self.resurrect_all
                || self.max_gold
                || self.max_food
                || self.refill_arrows
                || self.max_reagents
                || self.repair_hull
                || self.locks.any()
                || self.stat_edit
                || self.teleport)
    }

    /// Whether the Cheats page exists: some cheat or the watch is on
    /// (7.2.9). This is also the Ctrl+F12 binding rule (D-M8.5).
    pub fn any_on(&self) -> bool {
        self.state_watch || self.any_state_altering()
    }
}

impl ResolvedEnhancements {
    /// The `[cheats]` section as [`CheatToggles`] (7.3.2). A row that has
    /// not shipped has already been resolved to off (U13), and the parity
    /// lock gives the default, every field false (7.4.5).
    pub fn cheats(&self) -> CheatToggles {
        if matches!(self.lock, LockState::Locked(_)) {
            return CheatToggles::default();
        }
        let on = |toggle: Toggle| self.toggles.on(toggle);
        let enabled = on(Toggle::CheatsEnabled);
        let gated = |toggle: Toggle| enabled && on(toggle);
        CheatToggles {
            enabled,
            heal_all: gated(Toggle::CheatsHealAll),
            cure_poison: gated(Toggle::CheatsCurePoison),
            resurrect_all: gated(Toggle::CheatsResurrectAll),
            max_gold: gated(Toggle::CheatsMaxGold),
            max_food: gated(Toggle::CheatsMaxFood),
            refill_arrows: gated(Toggle::CheatsRefillArrows),
            refill_quarrels: gated(Toggle::CheatsRefillQuarrels),
            max_reagents: gated(Toggle::CheatsMaxReagents),
            repair_hull: gated(Toggle::CheatsRepairHull),
            locks: LockPermissions {
                health: gated(Toggle::CheatsLockHp),
                mana: gated(Toggle::CheatsLockMp),
                food: gated(Toggle::CheatsLockFood),
                gold: gated(Toggle::CheatsLockGold),
                keys: gated(Toggle::CheatsLockKeys),
                gems: gated(Toggle::CheatsLockGems),
                torches: gated(Toggle::CheatsLockTorches),
                arrows: gated(Toggle::CheatsLockArrows),
                reagents: gated(Toggle::CheatsLockReagents),
            },
            remember_locks: gated(Toggle::CheatsRememberLocks),
            stat_edit: gated(Toggle::CheatsStatEdit),
            teleport: gated(Toggle::CheatsTeleport),
            state_watch: on(Toggle::CheatsStateWatch),
        }
    }
}

#[cfg(test)]
mod tests;

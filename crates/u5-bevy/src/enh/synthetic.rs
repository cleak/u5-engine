//! The one owner of synthetic keys (Phase B design 1.3.7).
//!
//! Some features type keys for the player: the menu's slot save types `Q`
//! and `Y`, Sound and Buffer type their command, the start keys and the
//! title `J` type theirs, and automatic combat types each member's command.
//! Each of them is one [`SyntheticOwner`], and [`SyntheticInputOwner`] is
//! the one token they share:
//!
//! - One owner holds it at a time. Another owner's [`take`] is refused
//!   until the holder releases it.
//! - Only the holder queues keys.
//! - `feed_synthetic_keys`, in `EnhSet::Feed`, moves queued keys into
//!   the game's key queue (`VisualState::typed_keys`) through the same
//!   per-key path a typed key takes, so every rule, random draw and save
//!   follows exactly as if the player had typed them.
//! - No key is fed while an overlay is open: they wait for it to close.
//!
//! The token is in the app only once a synthetic-key feature installs it
//! ([`install_synthetic_input`]).
//!
//! [`take`]: SyntheticInputOwner::take

use std::collections::VecDeque;

use bevy::prelude::*;

use super::EnhSet;
use super::overlay::OverlayState;
use crate::{TypedKey, VisualState};

/// The features that type keys for the player.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SyntheticOwner {
    /// The menu's slot save, Sound and Buffer (chapter 3).
    MenuSlotSave,
    /// The keys typed at start (chapter 3).
    StartKeys,
    /// Automatic combat (chapter 6).
    AutoCombat,
    /// The title menu's `J` (chapter 3).
    TitleJ,
}

/// One synthetic keystroke, with the modifiers it is typed under.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SyntheticKey {
    pub key: KeyCode,
    pub shift: bool,
    pub control: bool,
}

impl SyntheticKey {
    /// `key` with no modifier.
    pub const fn plain(key: KeyCode) -> Self {
        Self {
            key,
            shift: false,
            control: false,
        }
    }

    /// `key` typed with Shift.
    pub const fn shifted(key: KeyCode) -> Self {
        Self {
            key,
            shift: true,
            control: false,
        }
    }

    fn typed(self) -> TypedKey {
        TypedKey {
            key: self.key,
            shift: self.shift,
            control: self.control,
        }
    }
}

/// Why the token refused an owner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SyntheticRefused {
    /// Another owner holds the token.
    HeldBy(SyntheticOwner),
    /// The owner does not hold the token.
    NotHolder,
}

/// The token every synthetic key source shares, and the keys its holder
/// has queued and not yet fed.
#[derive(Resource, Debug, Default)]
pub struct SyntheticInputOwner {
    holder: Option<SyntheticOwner>,
    queued: VecDeque<SyntheticKey>,
}

impl SyntheticInputOwner {
    /// The owner that holds the token.
    pub fn holder(&self) -> Option<SyntheticOwner> {
        self.holder
    }

    /// Whether `me` could hold the token: it is free or `me` holds it.
    pub fn free_or(&self, me: SyntheticOwner) -> bool {
        self.holder.is_none_or(|holder| holder == me)
    }

    /// Takes the token for `me`. Taking it again while holding it is fine.
    pub fn take(&mut self, me: SyntheticOwner) -> Result<(), SyntheticRefused> {
        match self.holder {
            Some(holder) if holder != me => Err(SyntheticRefused::HeldBy(holder)),
            _ => {
                self.holder = Some(me);
                Ok(())
            }
        }
    }

    /// Releases the token if `me` holds it, and drops the keys it queued and
    /// that were not fed, so no key outlives its owner. Returns whether `me`
    /// held it.
    pub fn release(&mut self, me: SyntheticOwner) -> bool {
        if self.holder != Some(me) {
            return false;
        }
        self.holder = None;
        self.queued.clear();
        true
    }

    /// Queues `keys` for the next feed, in order. Only the holder queues.
    pub fn queue(
        &mut self,
        me: SyntheticOwner,
        keys: impl IntoIterator<Item = SyntheticKey>,
    ) -> Result<(), SyntheticRefused> {
        match self.holder {
            Some(holder) if holder == me => {
                self.queued.extend(keys);
                Ok(())
            }
            Some(holder) => Err(SyntheticRefused::HeldBy(holder)),
            None => Err(SyntheticRefused::NotHolder),
        }
    }

    /// The keys queued and not yet fed.
    pub fn queued(&self) -> usize {
        self.queued.len()
    }

    /// Whether the next feed moves keys: some are queued and no overlay is
    /// open.
    pub fn may_feed(&self, overlay_open: bool) -> bool {
        !overlay_open && self.holder.is_some() && !self.queued.is_empty()
    }

    /// Takes the queued keys for the game, oldest first, unless an overlay
    /// is open.
    fn drain_for_feed(&mut self, overlay_open: bool) -> Vec<SyntheticKey> {
        if !self.may_feed(overlay_open) {
            return Vec::new();
        }
        self.queued.drain(..).collect()
    }
}

/// Adds the token and its feed, once. Every synthetic-key feature calls it
/// from its own install; nothing else does.
pub fn install_synthetic_input(app: &mut App) {
    if app.world().contains_resource::<SyntheticInputOwner>() {
        return;
    }
    app.init_resource::<SyntheticInputOwner>().add_systems(
        Update,
        feed_synthetic_keys
            .run_if(synthetic_keys_queued)
            .in_set(EnhSet::Feed),
    );
}

fn synthetic_keys_queued(owner: Res<SyntheticInputOwner>) -> bool {
    owner.queued() > 0
}

/// Moves the holder's queued keys into the game's key queue, in
/// `EnhSet::Feed`, after the overlay bookkeeping and before the input
/// driver reads. Nothing moves while an overlay is open, or before
/// gameplay (no `VisualState`): the keys wait.
pub(super) fn feed_synthetic_keys(
    mut owner: ResMut<SyntheticInputOwner>,
    overlay: Option<Res<OverlayState>>,
    visual: Option<ResMut<VisualState>>,
) {
    let Some(mut visual) = visual else {
        return;
    };
    let overlay_open = overlay.is_some_and(|overlay| overlay.is_open());
    let keys = owner.drain_for_feed(overlay_open);
    if keys.is_empty() {
        // Held behind an overlay: leave `VisualState` unmarked, so a
        // change-driven reader (the save watcher) sees no change.
        return;
    }
    visual
        .typed_keys
        .extend(keys.into_iter().map(SyntheticKey::typed));
}

#[cfg(test)]
mod tests;

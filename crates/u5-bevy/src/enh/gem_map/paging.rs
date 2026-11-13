//! PgUp and PgDn on a one-panel dungeon gem map (Phase B design 10.2.6,
//! 10.3.5 "Keys", D10.12; build plan M3-14, D-M3.6).
//!
//! With `gem_map.level_paging` on, the router is in the app even with no
//! other key bound (D-M3.6), its [`KeyMap`] empty. While the map shows a
//! one-panel dungeon view (the 4:3 canvas, or `gem_map.dungeon_stack` off)
//! [`gem_paging_keys`] binds PgUp and PgDn to [`EnhAction::GemPageUp`] and
//! [`EnhAction::GemPageDown`] ([`KeyMap::set_gem_paging`]), and releases them
//! on the frame the view closes, the map is not drawn or the toggle goes
//! off. The router honours them only as the first press of a frame with
//! nothing queued (the order rule, 1.2.6) and resets them on
//! `ButtonInput`, so the game never sees them: at that moment the game's
//! only use of a key is to close its view and throw the key away, so a
//! taken PgUp leaves the game exactly as a player who waited (10.4.3).
//! Every other PgUp and PgDn reaches the game as in 1.0.1.
//!
//! [`gem_page`] turns the page: the nearest level above or below with a
//! panel. The map is rendered again from the same open view; nothing of
//! the game is read or changed.

use bevy::prelude::*;
use u5_runtime::enhancements::{GemMapToggles, ResolvedEnhancements};

use super::{GEM, GemShow};
use crate::enh::input::{EnhAction, EnhActionEvent, KeyMap};
use crate::enh::{EnhLaunch, Toggles, VisualLaunchConfig, feature_wanted};

/// The launch's `[gem_map]` rows.
fn gem_rows(resolved: &ResolvedEnhancements) -> GemMapToggles {
    resolved.gem_map()
}

/// Whether the launch wants the paging keys: the gem map's plugin is added
/// and `gem_map.level_paging` is on. Only then is the router installed for
/// them and are the paging systems added.
pub(crate) fn paging_wanted(launch: &VisualLaunchConfig) -> bool {
    feature_wanted(launch.enhancements.toggles, GEM)
        && gem_rows(&launch.enhancements).opts.level_paging
}

/// Adds the paging systems when the launch wants them and the router is in
/// the app.
pub(super) fn add_paging(app: &mut App, launch: &VisualLaunchConfig) {
    if !paging_wanted(launch) || !app.world().contains_resource::<KeyMap>() {
        return;
    }
    app.add_systems(
        Update,
        (
            gem_page
                .before(super::view::gem_render)
                .run_if(on_event::<EnhActionEvent>)
                .run_if(crate::enh::toggle_on(GEM)),
            gem_paging_keys
                .after(super::view::gem_render)
                .after(super::view::gem_hide)
                .run_if(gem_paging_due),
        )
            .in_set(crate::enh::EnhSet::Present),
    );
}

/// Turns the page of the open one-panel dungeon map on each paging action.
fn gem_page(
    mut show: ResMut<GemShow>,
    mut actions: EventReader<EnhActionEvent>,
    launch: Res<EnhLaunch>,
) {
    let t = gem_rows(&launch.0.enhancements);
    for EnhActionEvent(action) in actions.read() {
        let up = match action {
            EnhAction::GemPageUp => true,
            EnhAction::GemPageDown => false,
            _ => continue,
        };
        if let Some(view) = show.view_mut() {
            view.turn_page(up, t.opts.dungeon_stack);
        }
    }
}

/// Run condition: the toggle is on, or the pair is still bound and must
/// be released.
fn gem_paging_due(toggles: Option<Res<Toggles>>, keymap: Res<KeyMap>) -> bool {
    toggles.is_some_and(|toggles| toggles.0.on(GEM)) || keymap.gem_paging()
}

/// Whether PgUp and PgDn page now: the toggle on and the map showing a
/// one-panel dungeon view of the game's open gem view.
pub(super) fn paging_now(show: &GemShow, toggle_on: bool, t: &GemMapToggles) -> bool {
    toggle_on
        && t.opts.level_paging
        && show.sweep.open
        && show
            .view()
            .is_some_and(|view| view.is_one_panel_dungeon(t.opts.dungeon_stack))
}

/// Binds the paging pair while a one-panel dungeon map shows, and releases
/// it otherwise. Writes the keymap only when that changes.
fn gem_paging_keys(
    show: Res<GemShow>,
    toggles: Option<Res<Toggles>>,
    launch: Res<EnhLaunch>,
    mut keymap: ResMut<KeyMap>,
) {
    let on = toggles.is_some_and(|toggles| toggles.0.on(GEM));
    let want = paging_now(&show, on, &gem_rows(&launch.0.enhancements));
    if keymap.gem_paging() != want {
        keymap.set_gem_paging(want);
    }
}

#[cfg(test)]
#[path = "paging_tests.rs"]
mod tests;

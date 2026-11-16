//! The cheats plugin in the Bevy shell (Phase B design 7.2.1-7.2.5, 7.3.4,
//! 7.3.5, 7.3.7; build plan M8-02).
//!
//! The rules are the runtime's ([`u5_runtime::cheats`]); this module is
//! the one place that calls them. Its parts:
//!
//! - [`apply`]: `cheat_apply`, in `EnhSet::Observe`, the single point every
//!   class (c) change goes through. It drains the page's queue in order,
//!   applies the engaged locks once per lock epoch, marks the session
//!   tainted and repaints the frame (7.3.4, 7.3.5).
//! - [`edit`]: Edit party and stores, the draft behind the page's edit
//!   face, sent on Apply as one change (7.2.6).
//! - [`page`]: the Cheats page, `OverlayId::Cheats`, opened by the
//!   `[keys] cheats` chord through the foundation's router, with its result
//!   line and the "Allow cheats?" confirmation (7.2.1, 7.2.3).
//! - [`teleport`]: the Teleport face, the page's one spoiler face (7.2.7);
//!   Enter queues the move for `cheat_apply`.
//! - [`watch`]: the change watch, `watch_record` in `EnhSet::Observe`
//!   under its own `state_watch` condition, read-only, and its face
//!   (7.2.8, 7.3.8).
//! - [`marker`]: the modified marker, `SaveMeta { cheats_used }` in the
//!   foundation's META sidecar, written on each new save of a tainted
//!   session and read when gameplay starts (7.3.7, D-M8.1).
//!
//! **Zero footprint.** The plugin is added only when some cheat or the
//! watch is effectively on at launch, or the menu overlay is on so a cheat
//! can be switched on later ([`cheats_wanted`]), and never under the parity
//! lock (the foundation plugin itself is absent then). Its resources,
//! [`CheatSession`] and [`page::CheatPage`], exist only while the toggles
//! in force turn them on, and every system runs only while they exist, so
//! with the menu overlay on and every cheat off nothing of it runs and no
//! resource of it exists. `CheatSession` exists only while a state-altering
//! cheat is effective (`CheatToggles::any_state_altering`): with only the
//! watch on there is no session and no `cheat_apply` work.

pub mod apply;
pub mod edit;
pub mod marker;
pub mod page;
pub mod teleport;
pub mod watch;

use bevy::prelude::*;
use u5_runtime::enhancements::{CheatToggles, FeatureToggles, Toggle};

use super::input::{EnhActionEvent, menu_overlay_on};
use super::overlay::{
    OverlayChanged, OverlayId, OverlayRequest, OverlayState, apply_overlay_requests,
};
use super::{EnhSet, Toggles, VisualLaunchConfig, toggle_on};
use crate::VisualState;
use crate::menu::pages::{MenuDoor, MenuDoors};

pub use apply::{CheatAction, LockEpoch, LockKind, QuickAction};
pub use marker::MarkerState;
pub use page::{CheatPage, PageMode};
pub use watch::{WATCH_CAPACITY, WatchFolder, WatchLog};

/// The section switch: every system that changes the game runs under it.
const SWITCH: Toggle = Toggle::CheatsEnabled;

/// The typed view of the toggles in force.
pub fn cheat_toggles(toggles: FeatureToggles) -> CheatToggles {
    CheatToggles::from_toggles(toggles)
}

/// Whether the Cheats page exists under `toggles`: some cheat or the watch
/// is effectively on (7.2.9). The menu's **Cheats...** item asks this
/// (D-M8.2).
pub fn page_available(toggles: FeatureToggles) -> bool {
    cheat_toggles(toggles).any_on()
}

/// The menu's **Cheats...** item (design 3.2.8, D-M8.2): a door on the top
/// menu, listed while [`page_available`]. Choosing it closes the menu and
/// opens the page on its main face, as Ctrl+F12 does.
pub const MENU_DOOR: MenuDoor = MenuDoor {
    label: crate::menu::text::CHEATS,
    opens: OverlayId::Cheats,
};

/// Whether the plugin belongs in the app: a cheat or the watch is on at
/// launch, or the menu overlay is on so one can be switched on later
/// (7.3.1, 7.4.1). The caller is never under the lock.
pub fn cheats_wanted(launch: &VisualLaunchConfig) -> bool {
    launch.enhancements.cheats().any_on() || menu_overlay_on(launch.enhancements.toggles)
}

/// Adds the plugin when [`cheats_wanted`]. Called from
/// [`super::EnhancementsPlugin`], after the overlay model and the save
/// watcher.
pub(super) fn add_cheats(app: &mut App, launch: &VisualLaunchConfig) {
    if cheats_wanted(launch) {
        if let Some(dir) = &launch.watch_dir {
            app.insert_resource(WatchFolder(dir.clone()));
        }
        app.add_plugins(CheatsPlugin);
    }
}

/// The session's cheat state (7.3.4). Exists only while a state-altering
/// cheat is effective.
#[derive(Resource, Debug, Default)]
pub struct CheatSession {
    /// The page's actions, applied in order by `cheat_apply`.
    pub queued: Vec<CheatAction>,
    /// The checked locks, in [`LockKind::ALL`] order. Session state: they
    /// start unchecked at every launch (D7.2).
    pub engaged: [bool; LockKind::COUNT],
    /// The last lock epoch the locks were applied in.
    pub last_epoch: Option<LockEpoch>,
    /// A class (c) cheat changed the game in this session, or the loaded
    /// save carries the marker.
    pub tainted: bool,
    /// What the loaded save's marker said (7.3.7).
    pub marker: MarkerState,
    /// A change was made where a repaint could draw randomness (the
    /// first-person dungeon): the next natural redraw shows it (7.3.5).
    pub repaint_deferred: bool,
    /// The marker could not be written; the next save tries again.
    pub marker_retry: bool,
    /// Marker reads asked for (gameplay start, a slot load).
    pub(crate) attach_requested: bool,
}

impl CheatSession {
    /// Whether `lock` is checked.
    pub fn is_engaged(&self, lock: LockKind) -> bool {
        self.engaged[lock as usize]
    }

    /// The header's word (7.2.3).
    pub fn header(&self) -> MarkerState {
        if self.tainted {
            MarkerState::Modified
        } else {
            self.marker
        }
    }

    /// A slot load rewrote the save: the next frame reads its marker.
    pub fn request_attach(&mut self) {
        self.attach_requested = true;
    }
}

/// Asks for the "Allow cheats?" confirmation (7.2.1). The menu sends it when
/// the player ticks **Allow cheats**; `Y` turns `cheats.enabled` on through
/// the foundation's pending changes.
#[derive(Event, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AllowCheatsRequest;

/// The cheats feature plugin.
pub struct CheatsPlugin;

impl Plugin for CheatsPlugin {
    fn build(&self, app: &mut App) {
        // The page needs the overlay model, which the foundation adds when
        // the cheats chord is bound or the menu is on; a hand-built
        // launch may have neither.
        if !app.world().contains_resource::<OverlayState>() {
            super::overlay::install_overlay_model(app);
        }
        app.add_event::<AllowCheatsRequest>()
            .add_systems(
                Update,
                cheats_sync
                    .run_if(resource_exists_and_changed::<Toggles>)
                    .in_set(EnhSet::Apply)
                    .before(apply_overlay_requests),
            )
            .add_systems(
                Update,
                cheats_door
                    .run_if(resource_exists::<MenuDoors>)
                    .in_set(EnhSet::Apply)
                    .before(apply_overlay_requests),
            )
            .add_systems(
                Update,
                cheats_door_opened
                    .run_if(resource_exists::<CheatPage>)
                    .in_set(EnhSet::Apply)
                    .after(apply_overlay_requests),
            )
            .add_systems(
                Update,
                cheats_allow
                    .run_if(on_event::<AllowCheatsRequest>)
                    .after(cheats_sync)
                    .before(cheats_open)
                    .in_set(EnhSet::Apply),
            )
            .add_systems(
                Update,
                (cheats_open, page::page_keys)
                    .chain()
                    .after(cheats_sync)
                    .before(apply_overlay_requests)
                    .in_set(EnhSet::Apply)
                    .run_if(resource_exists::<CheatPage>),
            )
            .add_systems(
                Update,
                apply::cheat_apply
                    .in_set(EnhSet::Observe)
                    .run_if(toggle_on(SWITCH))
                    .run_if(resource_exists::<CheatSession>)
                    .run_if(apply::state_altering_on)
                    .run_if(resource_exists::<VisualState>)
                    .run_if(super::idle::at_idle_prompt(
                        super::overlay::OverlayId::Cheats.kinds(),
                    )),
            )
            .add_systems(
                Update,
                watch::watch_record
                    .in_set(EnhSet::Observe)
                    .run_if(toggle_on(Toggle::CheatsStateWatch))
                    .run_if(resource_exists::<watch::WatchLog>)
                    .run_if(resource_exists::<VisualState>),
            )
            .add_systems(
                Update,
                page::page_present
                    .in_set(EnhSet::Present)
                    .run_if(page::page_present_due),
            );
        marker::add_marker(app);
    }
}

/// Inserts or removes the session and the page as the toggles change
/// (7.4.1): the session while a state-altering cheat is effective, the page
/// while any cheat or the watch is. The page closes with the last cheat.
fn cheats_sync(
    mut commands: Commands,
    toggles: Res<Toggles>,
    session: Option<Res<CheatSession>>,
    page: Option<Res<CheatPage>>,
    watch: Option<Res<WatchLog>>,
    overlay: Option<Res<OverlayState>>,
    mut requests: EventWriter<OverlayRequest>,
) {
    let cheats = cheat_toggles(toggles.0);
    // The watch's log exists while `state_watch` is on (7.3.8).
    match (cheats.state_watch, watch.is_some()) {
        (true, false) => commands.insert_resource(WatchLog::default()),
        (false, true) => commands.remove_resource::<WatchLog>(),
        _ => {}
    }
    match (cheats.any_state_altering(), session.is_some()) {
        (true, false) => commands.insert_resource(CheatSession {
            attach_requested: true,
            ..CheatSession::default()
        }),
        (false, true) => commands.remove_resource::<CheatSession>(),
        _ => {}
    }
    // A page asking the "Allow cheats?" question stays until answered.
    let page_wanted = cheats.any_on() || page.as_ref().is_some_and(|page| page.confirming());
    match (page_wanted, page.is_some()) {
        (true, false) => commands.insert_resource(CheatPage::default()),
        (false, true) => {
            commands.remove_resource::<CheatPage>();
            if overlay.is_some_and(|overlay| overlay.open() == Some(OverlayId::Cheats)) {
                requests.write(OverlayRequest::Close(OverlayId::Cheats));
            }
        }
        _ => {}
    }
}

/// Lists the menu's **Cheats...** door while the page is available, and
/// takes it away with the last cheat (3.2.8). The menu exists only with
/// `menu.overlay`, so without it this never runs.
fn cheats_door(toggles: Res<Toggles>, mut doors: ResMut<MenuDoors>) {
    let listed = doors.0.contains(&MENU_DOOR);
    match (page_available(toggles.0), listed) {
        (true, false) => doors.0.push(MENU_DOOR),
        (false, true) => doors.0.retain(|door| *door != MENU_DOOR),
        _ => {}
    }
}

/// The page opened through the menu's door: it shows its main face, as it
/// does for Ctrl+F12, unless it is asking "Allow cheats?".
fn cheats_door_opened(mut changed: EventReader<OverlayChanged>, mut page: ResMut<CheatPage>) {
    for change in changed.read() {
        if *change == OverlayChanged::Opened(OverlayId::Cheats) && !page.confirming() {
            page.open_main();
        }
    }
}

/// The menu asked for the "Allow cheats?" confirmation: the page is made if
/// no cheat had made it yet, and asked to open on the question (7.2.1).
fn cheats_allow(
    mut commands: Commands,
    mut allow: EventReader<AllowCheatsRequest>,
    page: Option<ResMut<CheatPage>>,
    mut requests: EventWriter<OverlayRequest>,
) {
    allow.clear();
    match page {
        Some(mut page) => page.open_confirm(),
        None => {
            let mut page = CheatPage::default();
            page.open_confirm();
            commands.insert_resource(page);
        }
    }
    requests.write(OverlayRequest::Open(OverlayId::Cheats));
}

/// The `[keys] cheats` chord asks the overlay model to open the page; it
/// opens only at an idle prompt (7.2.2). The page's own keys close it
/// ([`page::page_keys`]).
fn cheats_open(
    mut actions: EventReader<EnhActionEvent>,
    mut page: ResMut<CheatPage>,
    mut requests: EventWriter<OverlayRequest>,
) {
    for action in actions.read() {
        if action.0 == super::input::EnhAction::OpenCheats && !page.confirming() {
            page.open_main();
            requests.write(OverlayRequest::Open(OverlayId::Cheats));
        }
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod review_tests;

#[cfg(test)]
mod spoiler_tests;

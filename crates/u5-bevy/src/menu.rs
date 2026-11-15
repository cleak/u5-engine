//! The menu overlay (Phase B design chapter 3; build plan M4).
//!
//! The look ([`frame`], [`geometry`], [`text`]) is M4-01's. M4-04 adds the
//! plugin, the overlay wiring and the pure page model ([`pages`]):
//!
//! - [`MenuPlugin`] is added from the enhancements plugin only while
//!   `menu.overlay` or `menu.title_layer` is on at launch (3.4.2), never
//!   under the parity lock (the enhancements plugin itself is absent then).
//!   With both off none of this module is in the app.
//! - It has no input system of its own (3.3.4). The router binds F12 as
//!   `[keys] menu` and sends `OpenMenu` only as the first press of a frame
//!   with nothing queued; `menu_actions` turns it into
//!   [`OverlayRequest::Open`]`(Menu)`, which the overlay model grants at
//!   the idle exploration prompt and otherwise refuses with a toast. With
//!   only `title_layer` on, F12 in play is consumed by the router (it has
//!   no game byte) and ignored here (D-M4.3); the title layer opens through
//!   its own path in Phase 3.
//! - While the menu is open the router sends every key as an
//!   [`OverlayKey`] and every typed character as an [`OverlayText`];
//!   `menu_input` hands them to the reducer and carries out its commands.
//!   Closing and opening a door go through the overlay model; every
//!   command is also sent as a [`MenuCommandEvent`] for the packages that
//!   carry out the rest (Save, Load, the quits, settings).
//! - The QA key script (D-M4.6) is fed one key per frame, only while the
//!   menu is open ([`script_input`]).
//!
//! - M4-06 adds the settings plumbing ([`settings`]): the Settings rows
//!   from the generator the setup wizard uses, each with its effective
//!   apply point and command-line mark, and `Flip` and `Step` on a setting
//!   row queued as a change the foundation applies at that point.
//!
//! - M4-07 fills the pages: the Settings tabs' rows ([`settings::settings_tabs`])
//!   in every snapshot the pages are built from, the preset row, and Leave
//!   the game, which writes `AppExit` as the window's close button does and
//!   saves nothing.
//!
//! - M4-05 draws it ([`render`]): the page in the frame kit on a picture
//!   the size of the presented frame or canvas, an `ImageNode` exactly over
//!   the frame sprite, a full-window dim under it, and the open and close
//!   sweep.
//!
//! - M4-08 adds the output gain and focus mute ([`audio`]): `VolumePlugin`
//!   and `FocusMutePlugin`, each added by its own 3.4.2 condition (they
//!   work without the overlay), so the Audio tab's rows apply at once.
//!
//! - M4-10 adds the slot save's handshake ([`inject`]): the predicate that
//!   enables Save, the `Q`-then-`Y` typing through the synthetic-key owner,
//!   the check of the working save and the thumbnail ([`thumb`]). It is in
//!   the app only with named slots and a slot store.
//!
//! - M4-11 adds the Save and Load pages ([`slots`], drawn by
//!   [`render_slots`]): the host's slot store behind [`host::MenuHost`],
//!   the save through the handshake and the host's worker, the load by
//!   relaunch with the boot `J`, rename and delete, and the turns since
//!   the last save or load ([`unsaved`]). [`slots::SlotPlugin`] is in the
//!   app only with named slots and a slot store.
//!
//! - M4-12 adds the title layer ([`title`]): the menu key at the title menu
//!   opens its own page outside the overlay model (D-M4.3), whose Continue
//!   and Load type the title menu's `J` after the slot is restored; and
//!   Return to the title, which asks the host to start the game again with
//!   no key waiting, so the whole intro plays (3.2.5).
//!
//! - M4-13 adds the Game tab ([`settings::game_tab`]): Sound and Buffer
//!   type the game's own Ctrl-S and Ctrl-B ([`inject::install_game_keys`]);
//!   the start keys ([`start_keys`]), their own plugin that works without
//!   the overlay; and the unsaved-progress question before Load, Return to
//!   the title and Leave the game ([`pages::Unsaved`], 3.2.6). The menu's
//!   phases ship with it.
//!
//! The world stays Live (U1): the menu acts on the game only through the
//! overlay model, which hands keys back on close with held keys latched.

pub mod audio;
pub mod frame;
pub mod geometry;
pub mod host;
pub mod inject;
pub mod pages;
pub mod render;
pub mod render_slots;
pub mod settings;
pub mod slots;
pub mod start_keys;
pub mod text;
pub mod thumb;
pub mod title;
pub mod unsaved;

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use u5_runtime::enhancements::{Phase, SHIPPED_PHASES, Toggle};

use self::host::MenuHost;
use self::pages::{MenuCommand, MenuDoors, MenuInput, MenuModel, ShellSnapshot, Unsaved, reduce};
use self::slots::{SlotListing, SlotPlugin};
use self::title::{TitleJourney, title_open};
use self::unsaved::{SaveTracker, install_save_tracker};
use crate::enh::input::{
    EnhAction, EnhActionEvent, HotkeyFellThrough, KeyChord, OverlayKey, OverlayText,
};
use crate::enh::overlay::{
    OverlayChanged, OverlayId, OverlayRequest, OverlayState, apply_overlay_requests,
    close_when_not_idle,
};
use crate::enh::synthetic::{SyntheticInputOwner, install_synthetic_input};
use crate::enh::toast::Toasts;
use crate::enh::{
    EnhLaunch, EnhSet, LiveSettings, PendingSettingChanges, PendingToggleChanges, Relaunch,
    ShellExit, ShellExitSlot, VisualLaunchConfig, feature_wanted, toggle_on,
};
use crate::{VisualIntroState, VisualState};

/// The two toggles that bring the plugin (3.4.2).
const OVERLAY: Toggle = Toggle::MenuOverlay;
const TITLE_LAYER: Toggle = Toggle::MenuTitleLayer;

/// Adds [`MenuPlugin`] when `menu.overlay` or `menu.title_layer` is on at
/// launch; otherwise adds nothing. Called from the enhancements plugin
/// after the overlay model, which either toggle brings.
pub(crate) fn add_menu(app: &mut App, launch: &VisualLaunchConfig) {
    let toggles = launch.enhancements.toggles;
    if feature_wanted(toggles, OVERLAY) || feature_wanted(toggles, TITLE_LAYER) {
        let slots = launch.enhancements.menu().named_slots;
        app.add_plugins(MenuPlugin {
            overlay: feature_wanted(toggles, OVERLAY),
            title_layer: feature_wanted(toggles, TITLE_LAYER),
            host: launch.menu_host.clone().filter(|_| slots),
            script: launch.menu_script.clone().unwrap_or_default(),
            save_dir: launch.save_dir.clone(),
        });
    }
}

/// The menu's feature plugin: the page model, the doors and the systems
/// that wire them to the overlay model.
pub struct MenuPlugin {
    /// `menu.overlay` was wanted at launch: the Game tab's Sound and Buffer
    /// can type their keys.
    overlay: bool,
    /// `menu.title_layer` was wanted at launch: the title layer's systems
    /// are in the app (with the intro).
    title_layer: bool,
    /// The host's slot store, while named slots are on.
    host: Option<Arc<dyn MenuHost>>,
    /// The QA key script (D-M4.6), empty without one.
    script: Vec<char>,
    /// The folder the host chose for the save files, for the save watcher
    /// the unsaved-progress count reads.
    save_dir: Option<PathBuf>,
}

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        // The default launch builds the intro first; the `--visual`
        // harness shell has no intro and offers the short menu.
        let intro_shell = app.world().contains_resource::<VisualIntroState>();
        app.insert_resource(MenuShell {
            intro_shell,
            slots: self.host.is_some(),
            shipped: SHIPPED_PHASES,
        })
        .init_resource::<MenuState>()
        .init_resource::<MenuDoors>()
        .init_resource::<render::MenuView>()
        .add_event::<MenuCommandEvent>()
        .add_event::<TitleJourney>()
        .add_systems(
            Update,
            (menu_actions.run_if(toggle_on(OVERLAY)), menu_input)
                .chain()
                .in_set(EnhSet::Apply)
                .before(apply_overlay_requests),
        )
        .add_systems(
            Update,
            (
                settings::menu_setting_commands.before(crate::enh::apply_pending_changes),
                quit_to_desktop,
                quit_to_title,
            )
                .run_if(on_event::<MenuCommandEvent>)
                .after(menu_input)
                .in_set(EnhSet::Apply),
        )
        .add_systems(
            Update,
            (
                menu_follow_overlay.after(close_when_not_idle),
                menu_fell_through.run_if(toggle_on(OVERLAY)),
            )
                .in_set(EnhSet::Observe),
        )
        .add_systems(
            Update,
            (
                render::menu_render,
                render::place_overlay.run_if(render::place_due),
            )
                .chain()
                .in_set(EnhSet::Present),
        );
        if self.title_layer && intro_shell {
            install_synthetic_input(app);
            app.add_systems(
                Update,
                (
                    title::title_actions
                        .run_if(toggle_on(TITLE_LAYER))
                        .before(menu_input),
                    title::title_commands.after(menu_input),
                )
                    .in_set(EnhSet::Apply),
            )
            .add_systems(Update, title::title_follow_intro.in_set(EnhSet::Observe));
        }
        if self.overlay {
            inject::install_game_keys(app);
        }
        install_save_tracker(app, self.save_dir.clone());
        if let Some(host) = &self.host {
            app.add_plugins(SlotPlugin { host: host.clone() });
        }
        if !self.script.is_empty() {
            app.insert_resource(MenuScript(self.script.iter().copied().collect()))
                .add_systems(
                    Update,
                    feed_menu_script
                        .run_if(menu_open)
                        .run_if(script_left)
                        .in_set(EnhSet::Apply)
                        .before(menu_input),
                );
        }
    }
}

/// What the plugin knew at launch about the shell it is in.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct MenuShell {
    /// The default launch, with the intro; `false` in the `--visual`
    /// harness shell.
    pub intro_shell: bool,
    /// Save and Load are offered.
    pub slots: bool,
    /// The catalogue phases this build has shipped, whose rows the Settings
    /// tabs list: [`SHIPPED_PHASES`]. Tests and the review shots add the
    /// menu's own phases, which ship at the end of the milestone (M4-13).
    pub shipped: &'static [Phase],
}

/// The open menu's model; `None` while the menu is closed.
#[derive(Resource, Debug, Default)]
pub struct MenuState {
    model: Option<MenuModel>,
}

impl MenuState {
    /// The open menu's model.
    pub fn model(&self) -> Option<&MenuModel> {
        self.model.as_ref()
    }
}

/// A command the reducer gave. Closing and doors are carried out here;
/// the packages that own the rest read these.
#[derive(Event, Clone, Copy, Debug, PartialEq, Eq)]
pub struct MenuCommandEvent(pub MenuCommand);

/// The QA key script still to feed (D-M4.6).
#[derive(Resource, Debug, Default)]
struct MenuScript(VecDeque<char>);

fn menu_open(overlay: Option<Res<OverlayState>>, state: Res<MenuState>) -> bool {
    overlay.is_some_and(|overlay| overlay.open() == Some(OverlayId::Menu)) || title_open(&state)
}

fn script_left(script: Res<MenuScript>) -> bool {
    !script.0.is_empty()
}

/// F12 in play asks the overlay model for the menu (`menu.overlay` only).
/// Before gameplay (no `VisualState`) the action is the title layer's.
fn menu_actions(
    mut actions: EventReader<EnhActionEvent>,
    visual: Option<Res<VisualState>>,
    mut requests: EventWriter<OverlayRequest>,
) {
    for action in actions.read() {
        if action.0 == EnhAction::OpenMenu && visual.is_some() {
            requests.write(OverlayRequest::Open(OverlayId::Menu));
        }
    }
}

/// The shell state the pages read now, without the Settings rows.
pub(crate) fn snapshot(shell: &MenuShell, doors: &MenuDoors) -> ShellSnapshot {
    ShellSnapshot {
        intro_shell: shell.intro_shell,
        slots: shell.slots,
        // Filled from the game by `full_snapshot` (M4-10).
        save_available: false,
        doors: doors.0.clone(),
        settings: Default::default(),
        quicksave: false,
        slot_list: Default::default(),
        turns_since_save: None,
        unsaved: None,
    }
}

/// What the pages are built from: the shell, the doors, the settings in
/// play with the changes still queued, and the game's own values.
#[derive(SystemParam)]
pub(crate) struct SnapshotInputs<'w> {
    shell: Res<'w, MenuShell>,
    doors: Res<'w, MenuDoors>,
    live: Option<Res<'w, LiveSettings>>,
    launch: Option<Res<'w, EnhLaunch>>,
    toggles: Option<Res<'w, PendingToggleChanges>>,
    settings: Option<Res<'w, PendingSettingChanges>>,
    visual: Option<Res<'w, VisualState>>,
    owner: Option<Res<'w, SyntheticInputOwner>>,
    listing: Option<Res<'w, SlotListing>>,
    tracker: Option<Res<'w, SaveTracker>>,
}

impl SnapshotInputs<'_> {
    /// The pages' snapshot, the Settings tabs' rows included (M4-07).
    pub(crate) fn snapshot(&self) -> ShellSnapshot {
        full_snapshot(
            &self.shell,
            &self.doors,
            self.live.as_deref(),
            self.launch.as_deref(),
            self.toggles.as_deref(),
            self.settings.as_deref(),
            self.visual.as_deref(),
            self.owner.as_deref(),
            self.listing.as_deref(),
            self.tracker.as_deref(),
        )
    }
}

/// [`SnapshotInputs::snapshot`] read straight from `world`, for tests.
#[cfg(test)]
pub(crate) fn world_snapshot(world: &World) -> ShellSnapshot {
    full_snapshot(
        world.resource::<MenuShell>(),
        world.resource::<MenuDoors>(),
        world.get_resource::<LiveSettings>(),
        world.get_resource::<EnhLaunch>(),
        world.get_resource::<PendingToggleChanges>(),
        world.get_resource::<PendingSettingChanges>(),
        world.get_resource::<VisualState>(),
        world.get_resource::<SyntheticInputOwner>(),
        world.get_resource::<SlotListing>(),
        world.get_resource::<SaveTracker>(),
    )
}

#[allow(clippy::too_many_arguments)]
fn full_snapshot(
    shell: &MenuShell,
    doors: &MenuDoors,
    live: Option<&LiveSettings>,
    launch: Option<&EnhLaunch>,
    toggles: Option<&PendingToggleChanges>,
    settings: Option<&PendingSettingChanges>,
    visual: Option<&VisualState>,
    owner: Option<&SyntheticInputOwner>,
    listing: Option<&SlotListing>,
    tracker: Option<&SaveTracker>,
) -> ShellSnapshot {
    let mut snap = snapshot(shell, doors);
    if let Some(listing) = listing {
        snap.slot_list = listing.slots.clone();
    }
    snap.turns_since_save = visual
        .zip(tracker)
        .and_then(|(visual, tracker)| tracker.turns_since(visual.state.turn));
    let menu = live.map_or_else(
        || launch.map(|launch| launch.0.enhancements.menu()),
        |live| Some(live.now.menu()),
    );
    snap.unsaved = unsaved(
        menu.is_some_and(|menu| menu.confirm_unsaved),
        visual.is_some(),
        snap.turns_since_save,
    );
    snap.quicksave = live.map_or_else(
        || launch.is_some_and(|launch| launch.0.enhancements.menu().quicksave),
        |live| live.now.menu().quicksave,
    );
    // The menu is open only at the idle exploration prompt, so `can_save`'s
    // idle half holds while the pages are built (3.2.2).
    snap.save_available = visual
        .is_some_and(|visual| owner.is_some_and(|owner| inject::can_save(visual, owner, true)));
    if let (Some(live), Some(launch), Some(toggles), Some(settings)) =
        (live, launch, toggles, settings)
    {
        let view = settings::settings_view_in(
            live,
            launch.0.enhancements.toggles,
            toggles,
            settings,
            shell.shipped,
        );
        let game = visual.map(|visual| settings::GameValues {
            sound_on: visual.state.music_enabled,
            buffer_on: visual.state.typeahead_buffer_enabled,
        });
        snap.settings = settings::settings_tabs(&view, game);
    }
    snap
}

/// What leaving now would lose (3.2.6): nothing to ask with the warning
/// off or before play; otherwise the turns since the last save or load,
/// none when the turn has not moved, and "this session" when no save or
/// load was seen.
pub fn unsaved(confirm: bool, in_play: bool, turns_since_save: Option<u64>) -> Option<Unsaved> {
    if !confirm || !in_play {
        return None;
    }
    match turns_since_save {
        Some(0) => None,
        Some(turns) => Some(Unsaved::Turns(turns)),
        None => Some(Unsaved::Session),
    }
}

/// Leave the game (3.2.5): the app exits as the window's close button
/// makes it, and nothing is saved. The reducer has asked first when there
/// was unsaved progress (3.2.6).
fn quit_to_desktop(mut commands: EventReader<MenuCommandEvent>, mut exit: EventWriter<AppExit>) {
    for MenuCommandEvent(command) in commands.read() {
        if *command == MenuCommand::QuitToDesktop {
            exit.write(AppExit::Success);
        }
    }
}

/// Return to the title (3.2.5): the app ends asking its host to start the
/// game again with the same arguments and no key waiting, so the whole
/// intro plays. Nothing is saved. The reducer has asked first when there
/// was unsaved progress (3.2.6). Only the relaunch exists in this build (`menu.quit_to_title`'s
/// in-app choice is chapter 3 Phase 7), so it is used whatever that says.
fn quit_to_title(
    mut commands: EventReader<MenuCommandEvent>,
    exit_slot: Option<Res<ShellExitSlot>>,
    mut exit: EventWriter<AppExit>,
    mut toasts: Option<ResMut<Toasts>>,
) {
    for MenuCommandEvent(command) in commands.read() {
        if *command != MenuCommand::QuitToTitle {
            continue;
        }
        match exit_slot.as_deref() {
            Some(slot) => {
                slot.set(quit_to_title_exit());
                exit.write(AppExit::Success);
            }
            None => {
                if let Some(toasts) = toasts.as_deref_mut() {
                    toasts.short(text::CANNOT_RESTART);
                }
            }
        }
    }
}

/// What Return to the title hands the host: a relaunch with no boot key.
pub fn quit_to_title_exit() -> ShellExit {
    ShellExit {
        relaunch: Some(Relaunch { boot_key: None }),
    }
}

/// Hands this frame's menu keys to the reducer and carries out what it
/// asks: the frame's typed text first, then its keys, each in order. It
/// runs every frame so that keys meant for another overlay are read and
/// dropped, never left for the menu's next opening.
fn menu_input(
    mut keys: EventReader<OverlayKey>,
    mut text: EventReader<OverlayText>,
    mut state: ResMut<MenuState>,
    pages: SnapshotInputs,
    mut requests: EventWriter<OverlayRequest>,
    mut commands: EventWriter<MenuCommandEvent>,
) {
    let inputs: Vec<MenuInput> = text
        .read()
        .copied()
        .map(MenuInput::from)
        .chain(keys.read().copied().map(MenuInput::from))
        .collect();
    let Some(model) = state.model.as_mut() else {
        return;
    };
    if inputs.is_empty() {
        return;
    }
    let snap = pages.snapshot();
    for input in inputs {
        for command in reduce(model, input, &snap) {
            match command {
                // The title layer is not an overlay; it closes itself.
                MenuCommand::Close if !model.is_title() => {
                    requests.write(OverlayRequest::Close(OverlayId::Menu));
                }
                MenuCommand::OpenDoor(id) => {
                    requests.write(OverlayRequest::Open(id));
                }
                _ => {}
            }
            commands.write(MenuCommandEvent(command));
        }
    }
}

/// Starts the model when the menu opens and drops it when it closes, for
/// whatever reason (a command, a door, the game leaving the prompt).
fn menu_follow_overlay(mut changed: EventReader<OverlayChanged>, mut state: ResMut<MenuState>) {
    for change in changed.read() {
        match *change {
            OverlayChanged::Opened(OverlayId::Menu) => state.model = Some(MenuModel::new()),
            OverlayChanged::Closed {
                id: OverlayId::Menu,
                ..
            } => state.model = None,
            _ => {}
        }
    }
}

/// F12 that broke the first-key rule went to the game, which dropped it;
/// the toast says where the menu opens (1.2.6).
fn menu_fell_through(
    mut fell: EventReader<HotkeyFellThrough>,
    visual: Option<Res<VisualState>>,
    mut toasts: ResMut<Toasts>,
) {
    for event in fell.read() {
        if event.0 == EnhAction::OpenMenu && visual.is_some() {
            toasts.short(text::OPENS_AT_PROMPT);
        }
    }
}

/// Feeds the next key of the QA script, one per frame while the menu is
/// open, as the router would send it.
fn feed_menu_script(
    mut script: ResMut<MenuScript>,
    mut keys: EventWriter<OverlayKey>,
    mut text: EventWriter<OverlayText>,
) {
    let Some(ch) = script.0.pop_front() else {
        return;
    };
    let (key, typed) = script_input(ch);
    if let Some(key) = key {
        keys.write(key);
    }
    if let Some(ch) = typed {
        text.write(OverlayText(ch));
    }
}

/// What one character of the QA script presses (D-M4.6): Escape `\x1b`,
/// Enter `\r` or `\n`, Tab `\t`, Backspace `\x08`, Delete `\x7f`, F12
/// `\x0c` (form feed), and the arrows `↑ ↓ ← →`. A letter or digit presses
/// its key, with Shift for a capital, and types itself; any other
/// printable ASCII character only types itself. Anything else does
/// nothing.
pub fn script_input(ch: char) -> (Option<OverlayKey>, Option<char>) {
    let plain = |key| (Some(OverlayKey::plain(key)), None);
    match ch {
        '\u{1b}' => plain(KeyCode::Escape),
        '\r' | '\n' => plain(KeyCode::Enter),
        '\t' => plain(KeyCode::Tab),
        '\u{8}' => plain(KeyCode::Backspace),
        '\u{7f}' => plain(KeyCode::Delete),
        '\u{c}' => plain(KeyCode::F12),
        '↑' => plain(KeyCode::ArrowUp),
        '↓' => plain(KeyCode::ArrowDown),
        '←' => plain(KeyCode::ArrowLeft),
        '→' => plain(KeyCode::ArrowRight),
        ' ' => (Some(OverlayKey::plain(KeyCode::Space)), Some(' ')),
        _ if ch.is_ascii_alphanumeric() => {
            let key = KeyChord::parse(&ch.to_ascii_uppercase().to_string()).map(|chord| chord.key);
            (
                key.map(|key| OverlayKey {
                    key,
                    shift: ch.is_ascii_uppercase(),
                    ctrl: false,
                }),
                Some(ch),
            )
        }
        _ if (' '..='~').contains(&ch) => (None, Some(ch)),
        _ => (None, None),
    }
}

#[cfg(test)]
mod sheet_screens;

#[cfg(test)]
mod frame_tests;

#[cfg(test)]
mod sheet_tests;

#[cfg(test)]
mod plugin_tests;

#[cfg(test)]
mod review_tests;

#[cfg(test)]
mod pages_settings_tests;

#[cfg(test)]
mod inject_review_tests;

#[cfg(test)]
mod render_slots_tests;

#[cfg(test)]
mod game_tab_tests;

#[cfg(test)]
mod game_tab_review_tests;

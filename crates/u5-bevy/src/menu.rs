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
//! The world stays Live (U1): the menu acts on the game only through the
//! overlay model, which hands keys back on close with held keys latched.

pub mod audio;
pub mod frame;
pub mod geometry;
pub mod host;
pub mod pages;
pub mod render;
pub mod settings;
pub mod text;

use std::collections::VecDeque;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use u5_runtime::enhancements::{Phase, SHIPPED_PHASES, Toggle};

use self::pages::{MenuCommand, MenuDoors, MenuInput, MenuModel, ShellSnapshot, reduce};
use crate::enh::input::{
    EnhAction, EnhActionEvent, HotkeyFellThrough, KeyChord, OverlayKey, OverlayText,
};
use crate::enh::overlay::{
    OverlayChanged, OverlayId, OverlayRequest, OverlayState, apply_overlay_requests,
    close_when_not_idle,
};
use crate::enh::toast::Toasts;
use crate::enh::{
    EnhLaunch, EnhSet, LiveSettings, PendingSettingChanges, PendingToggleChanges,
    VisualLaunchConfig, feature_wanted, toggle_on,
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
        app.add_plugins(MenuPlugin {
            slots: launch.enhancements.menu().named_slots && launch.menu_host.is_some(),
            script: launch.menu_script.clone().unwrap_or_default(),
        });
    }
}

/// The menu's feature plugin: the page model, the doors and the systems
/// that wire them to the overlay model.
pub struct MenuPlugin {
    /// Named slots are on and the host keeps a slot store.
    slots: bool,
    /// The QA key script (D-M4.6), empty without one.
    script: Vec<char>,
}

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        // The default launch builds the intro first; the `--visual`
        // harness shell has no intro and offers the short menu.
        let intro_shell = app.world().contains_resource::<VisualIntroState>();
        app.insert_resource(MenuShell {
            intro_shell,
            slots: self.slots,
            shipped: SHIPPED_PHASES,
        })
        .init_resource::<MenuState>()
        .init_resource::<MenuDoors>()
        .init_resource::<render::MenuView>()
        .add_event::<MenuCommandEvent>()
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

fn menu_open(overlay: Option<Res<OverlayState>>) -> bool {
    overlay.is_some_and(|overlay| overlay.open() == Some(OverlayId::Menu))
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
        // `save_command_available` arrives with the Save page (M4-10).
        save_available: false,
        doors: doors.0.clone(),
        settings: Default::default(),
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
    )
}

fn full_snapshot(
    shell: &MenuShell,
    doors: &MenuDoors,
    live: Option<&LiveSettings>,
    launch: Option<&EnhLaunch>,
    toggles: Option<&PendingToggleChanges>,
    settings: Option<&PendingSettingChanges>,
    visual: Option<&VisualState>,
) -> ShellSnapshot {
    let mut snap = snapshot(shell, doors);
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

/// Leave the game (3.2.5): the app exits as the window's close button
/// makes it, and nothing is saved. The unsaved-progress check comes with
/// M4-13.
fn quit_to_desktop(mut commands: EventReader<MenuCommandEvent>, mut exit: EventWriter<AppExit>) {
    for MenuCommandEvent(command) in commands.read() {
        if *command == MenuCommand::QuitToDesktop {
            exit.write(AppExit::Success);
        }
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
                MenuCommand::Close => {
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

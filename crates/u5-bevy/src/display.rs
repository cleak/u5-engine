//! The display chapter's shell side (Phase B design chapter 2, 2.3.1 and
//! 2.3.4): scaling, fullscreen, side fill and the grow-left layout.
//!
//! This module is the seam the later packages fill. For now it holds:
//!
//! - [`Display`], the resolved `[display]` settings as a resource;
//! - the scale policies ([`scale`], M1-02), in the foundation's frame fit,
//!   and under `sharp` the `[prescale]` pipeline step (M1-03);
//! - borderless fullscreen and F11 (M1-03): [`WindowWish`], flipped by the
//!   router's `ToggleFullscreen` and put on the window by
//!   `apply_window_mode`, present only while `fullscreen`, `fullscreen_key`
//!   or `vsync` is away from its default;
//! - [`DisplayPlugin`], added inside the foundation's
//!   [`EnhancementsPlugin`](crate::EnhancementsPlugin) only when a display
//!   toggle is on at launch or the menu overlay is on (so the menu could
//!   switch one on later). With every key at its default it registers no
//!   system; each system a package adds is added only when its setting
//!   asks for it and sits in one of the foundation's
//!   [`EnhSet`](crate::EnhSet)s;
//! - the side fill ([`fill`], M1-04): solid, ribbon or bezel nodes in the
//!   bars beside the 4:3 frame, present only while a fill is chosen (or the
//!   menu overlay could choose one);
//! - [`initial_window`], which sets the window's first mode and present
//!   mode from the settings in both `WindowPlugin` blocks. With every key at
//!   its default it returns the window it was given, so the 1.0.1 window is
//!   unchanged.
//!
//! Under Classic, and under the parity lock, the foundation plugin is not
//! in the app, so neither is this one: no display system, resource or key
//! binding exists. The shell never reads `config.toml`, the environment or
//! the command line for any of this; the host resolves it.

use bevy::prelude::*;
use bevy::window::{MonitorSelection, PresentMode, WindowMode};
use u5_runtime::enhancements::{
    BezelMailbox, DISPLAY_TOGGLES, DisplaySettings, FeatureToggles, LockState, PillarFill,
};

use crate::EnhSet;
use crate::enh::VisualLaunchConfig;
use crate::enh::input::{EnhAction, EnhActionEvent};

pub mod fill;
pub mod scale;

/// The resolved `[display]` settings (2.3.2). Exists only while
/// [`DisplayPlugin`] is in the app.
#[derive(Resource, Clone, Debug, PartialEq, Eq)]
pub struct Display(pub DisplaySettings);

/// The display chapter's plugin. Add it through [`add_display`], which
/// leaves it out unless a display toggle (or the menu overlay) is on.
pub struct DisplayPlugin {
    settings: DisplaySettings,
    bezel_updates: Option<BezelMailbox>,
}

impl DisplayPlugin {
    pub fn new(settings: DisplaySettings) -> Self {
        Self {
            settings,
            bezel_updates: None,
        }
    }

    /// Where the host posts a bezel chosen while the game runs (2.3.3).
    pub fn with_bezel_updates(mut self, updates: Option<BezelMailbox>) -> Self {
        self.bezel_updates = updates;
        self
    }
}

impl Plugin for DisplayPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Display(self.settings.clone()));
        // Scaling, fullscreen, the side fill and the grow-left compositor
        // register here, each only when its setting asks for it and in an
        // `EnhSet`, as their packages land (2.3.4).
        scale::add_scale(app, &self.settings);
        add_window_mode(app, &self.settings);
        fill::add_fill(app, &self.settings, self.bezel_updates.clone());
    }
}

/// The window the session wants: borderless fullscreen or not (the
/// `display.fullscreen` setting, flipped by F11 for the session), and vsync.
/// Exists only while a window key is away from its default
/// ([`window_keys_on`]).
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowWish {
    pub fullscreen: bool,
    pub vsync: bool,
}

impl WindowWish {
    pub fn of(settings: &DisplaySettings) -> Self {
        Self {
            fullscreen: settings.fullscreen,
            vsync: settings.vsync,
        }
    }

    /// The window mode it asks for. Windowed is the 1.0.1 mode.
    pub fn mode(self) -> WindowMode {
        if self.fullscreen {
            WindowMode::BorderlessFullscreen(MonitorSelection::Current)
        } else {
            WindowMode::Windowed
        }
    }

    /// The present mode it asks for. Fifo, Bevy's default, is the 1.0.1
    /// mode.
    pub fn present_mode(self) -> PresentMode {
        if self.vsync {
            PresentMode::Fifo
        } else {
            PresentMode::AutoNoVsync
        }
    }
}

/// Whether a window key is away from its default, so the window systems
/// belong in the app: `fullscreen`, `fullscreen_key` (F11) or `vsync`.
pub fn window_keys_on(settings: &DisplaySettings) -> bool {
    settings.fullscreen || settings.fullscreen_key || !settings.vsync
}

/// The window systems (2.3.4), only for a window key away from its default:
/// `follow_window_settings` and `apply_window_mode` in `EnhSet::Apply`, and
/// `display_actions` for F11 while the router sends actions.
fn add_window_mode(app: &mut App, settings: &DisplaySettings) {
    if !window_keys_on(settings) {
        return;
    }
    app.insert_resource(WindowWish::of(settings));
    // F11 reaches this plugin only as the router's `ToggleFullscreen`. The
    // router is in the app exactly while a binding is active; when the
    // `[keys]` chord was refused there is none, and nothing to react to.
    let routed =
        settings.fullscreen_key && app.world().contains_resource::<Events<EnhActionEvent>>();
    if routed {
        app.add_systems(
            Update,
            display_actions
                .run_if(on_event::<EnhActionEvent>)
                .before(follow_window_settings)
                .in_set(EnhSet::Apply),
        );
    }
    app.add_systems(
        Update,
        (
            follow_window_settings.run_if(resource_changed::<Display>),
            apply_window_mode.run_if(resource_changed::<WindowWish>),
        )
            .chain()
            .in_set(EnhSet::Apply),
    );
}

/// F11 (`ToggleFullscreen`) flips the session's fullscreen. It does not need
/// `display.fullscreen` to be on, and it writes no setting (2.2.10).
fn display_actions(mut actions: EventReader<EnhActionEvent>, mut wish: ResMut<WindowWish>) {
    for action in actions.read() {
        if action.0 == EnhAction::ToggleFullscreen {
            wish.fullscreen = !wish.fullscreen;
        }
    }
}

/// A changed `display.fullscreen` or `display.vsync` (the menu overlay)
/// re-sets the wish; a change to any other display key leaves an F11 choice
/// alone.
fn follow_window_settings(
    display: Res<Display>,
    mut last: Local<Option<WindowWish>>,
    mut wish: ResMut<WindowWish>,
) {
    let settings = WindowWish::of(&display.0);
    let Some(previous) = last.replace(settings) else {
        // The wish was made from the launch settings.
        return;
    };
    if previous.fullscreen != settings.fullscreen {
        wish.fullscreen = settings.fullscreen;
    }
    if previous.vsync != settings.vsync {
        wish.vsync = settings.vsync;
    }
}

/// Puts the wish on the window (borderless fullscreen, 2.3.4). A window
/// already in the wanted mode is not touched, so the first frame leaves the
/// window [`initial_window`] opened. If the platform refuses the mode the
/// window stays as it is and the setting is left as chosen (2.2.11).
fn apply_window_mode(wish: Res<WindowWish>, mut windows: Query<&mut Window>) {
    let (mode, present_mode) = (wish.mode(), wish.present_mode());
    for mut window in &mut windows {
        if window.mode != mode {
            window.mode = mode;
        }
        if window.present_mode != present_mode {
            window.present_mode = present_mode;
        }
    }
}

/// Whether [`DisplayPlugin`] belongs in the app at launch: a display toggle
/// is on, or the menu overlay is (2.3.4). Sub-options of an off feature,
/// such as `display.crawl`, switch no toggle, so they never add it.
pub fn display_wanted(toggles: FeatureToggles) -> bool {
    DISPLAY_TOGGLES.iter().any(|toggle| toggles.on(*toggle))
        || crate::enh::input::menu_overlay_on(toggles)
}

/// Adds [`DisplayPlugin`] when [`display_wanted`] says so. Called from the
/// foundation's plugin, which is itself absent under Classic and the lock.
pub(crate) fn add_display(app: &mut App, launch: &VisualLaunchConfig) {
    if display_wanted(launch.enhancements.toggles) {
        let settings = launch_display(launch);
        if !scale::ScalePolicy::of(&settings).is_faithful() {
            // The frame fit belongs to the arbiter; with no side panel at
            // launch it is installed here, with the file's arbiter settings.
            crate::enh::side::install_side_space(
                app,
                crate::enh::side::SideSpaceSettings::from_values(&launch.enhancements.values),
            );
        }
        app.add_plugins(
            DisplayPlugin::new(settings).with_bezel_updates(launch.bezel_updates.clone()),
        );
    }
}

/// The launch's `[display]` settings with the bezel the host decoded
/// (2.3.3). Without one a bezel fill shows black bars.
pub fn launch_display(launch: &VisualLaunchConfig) -> DisplaySettings {
    let mut settings = launch.enhancements.display();
    if let PillarFill::Bezel(image) = &mut settings.pillar_fill {
        image.clone_from(&launch.bezel);
    }
    settings
}

/// The window a visual launch opens: `window`, the 1.0.1 window, with the
/// first window mode and present mode the settings ask for (2.3.4). Only a
/// key away from its default changes anything, and the parity lock leaves
/// every key at its default, so under Classic and the lock this is
/// `window` itself.
pub fn initial_window(mut window: Window, launch: &VisualLaunchConfig) -> Window {
    if launch.enhancements.lock != LockState::Off {
        return window;
    }
    let settings = launch.enhancements.display();
    if settings.fullscreen {
        window.mode = WindowMode::BorderlessFullscreen(MonitorSelection::Current);
    }
    if !settings.vsync {
        window.present_mode = PresentMode::AutoNoVsync;
    }
    window
}

#[cfg(test)]
mod tests;

//! The display chapter's shell side (Phase B design chapter 2, 2.3.1 and
//! 2.3.4): scaling, fullscreen, side fill and the grow-left layout.
//!
//! This module is the seam the later packages fill. For now it holds:
//!
//! - [`Display`], the resolved `[display]` settings as a resource;
//! - the scale policies ([`scale`], M1-02), in the foundation's frame fit;
//! - [`DisplayPlugin`], added inside the foundation's
//!   [`EnhancementsPlugin`](crate::EnhancementsPlugin) only when a display
//!   toggle is on at launch or the menu overlay is on (so the menu could
//!   switch one on later). With every key at its default it registers no
//!   system; each system a package adds is added only when its setting
//!   asks for it and sits in one of the foundation's
//!   [`EnhSet`](crate::EnhSet)s;
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
use u5_runtime::enhancements::{DISPLAY_TOGGLES, DisplaySettings, FeatureToggles, LockState};

use crate::enh::VisualLaunchConfig;

pub mod scale;

/// The resolved `[display]` settings (2.3.2). Exists only while
/// [`DisplayPlugin`] is in the app.
#[derive(Resource, Clone, Debug, PartialEq, Eq)]
pub struct Display(pub DisplaySettings);

/// The display chapter's plugin. Add it through [`add_display`], which
/// leaves it out unless a display toggle (or the menu overlay) is on.
pub struct DisplayPlugin {
    settings: DisplaySettings,
}

impl DisplayPlugin {
    pub fn new(settings: DisplaySettings) -> Self {
        Self { settings }
    }
}

impl Plugin for DisplayPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Display(self.settings.clone()));
        // Scaling, fullscreen, the side fill and the grow-left compositor
        // register here, each only when its setting asks for it and in an
        // `EnhSet`, as their packages land (2.3.4).
        scale::add_scale(app, &self.settings);
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
        let settings = launch.enhancements.display();
        if !scale::ScalePolicy::of(&settings).is_faithful() {
            // The frame fit belongs to the arbiter; with no side panel at
            // launch it is installed here, with the file's arbiter settings.
            crate::enh::side::install_side_space(
                app,
                crate::enh::side::SideSpaceSettings::from_values(&launch.enhancements.values),
            );
        }
        app.add_plugins(DisplayPlugin::new(settings));
    }
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

//! The menu's host services (design 3.3.1).
//!
//! The shell never touches the file system for slots, the clock or the
//! OS: the host hands it an implementation of [`MenuHost`] in
//! [`VisualLaunchConfig::menu_host`](crate::VisualLaunchConfig::menu_host).
//! u5-tui implements it over u5-host-desktop, as `DesktopWizardHost` does
//! for the wizard; a phone or console port supplies its own.
//!
//! M4-03 adds the trait empty, with the launch plumbing; the slot
//! operations arrive with the slot pages (M4-11).

/// What the menu asks of its host. Empty until the slot pages land.
pub trait MenuHost: Send + Sync {}

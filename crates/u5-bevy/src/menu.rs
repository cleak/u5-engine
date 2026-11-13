//! The menu overlay (Phase B design chapter 3; build plan M4).
//!
//! M4-01 adds the look only and registers nothing: the frame kit
//! ([`frame`]), where the panels sit on every picture ([`geometry`]) and
//! the wording ([`text`]). The pages, the reducer, the plugin and the host
//! trait follow in later packages; with every chapter 3 key at its default
//! none of this is in the app.

pub mod frame;
pub mod geometry;
pub mod host;
pub mod text;

#[cfg(test)]
mod sheet_screens;

#[cfg(test)]
mod frame_tests;

#[cfg(test)]
mod sheet_tests;

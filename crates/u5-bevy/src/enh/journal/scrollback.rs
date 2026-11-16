//! The conversation scrollback panel (Phase B design 8.2.6, 8.3.11; build
//! plan M6-04).
//!
//! `journal.clue_scrollback` registers a [`ConversationPanelProvider`]
//! (`PanelKind::Conversation`) in the foundation's [`PanelRegistry`]. The
//! arbiter places it after the journal panel (`display.panel_priority`'s
//! default `[journal, conversation]`), and while chapter 2's grow-left
//! canvas leaves no bars it hides the panel with its one-time toast.
//!
//! `scrollback_accumulate`, in `EnhSet::Observe`, steps the pure
//! [`Scrollback`] (its own transcript cursor) and copies the rows into the
//! provider. The panel shows the current conversation only: every row
//! since the Talk command's echo, the newest at the foot, runic cells in
//! `RUNES.CH`. The rows are cleared when the conversation ends and never
//! saved. It needs neither the clue log nor its observer.
//!
//! **Zero footprint.** Nothing is added unless the side-space arbiter is in
//! the app (a side-claiming toggle was on at launch); every resource and
//! the provider exist only while `clue_scrollback` is on.

use std::sync::{Arc, Mutex};

use bevy::prelude::*;
use u5_runtime::PlayState;
use u5_runtime::enhancements::Toggle;
use u5_runtime::enhancements::clues::{Scrollback, TextRow};

use crate::VisualState;
use crate::enh::side::{PANEL_BODY_ROWS, PanelContent, PanelKind, PanelRegistry, PanelRows};
use crate::enh::side::{SidePanelProvider, SidePreference};
use crate::enh::{EnhSet, Toggles, VisualLaunchConfig};

/// The scrollback's toggle.
pub const SCROLLBACK: Toggle = Toggle::JournalClueScrollback;

/// The panel's title row. Our own word.
pub const SCROLLBACK_TITLE: &str = "TALK";

/// The rows the provider shows, shared with [`scrollback_accumulate`].
/// `None` outside a conversation.
pub type SharedRows = Arc<Mutex<Option<Vec<TextRow>>>>;

/// The accumulator and the rows it shares. Present exactly while the
/// provider is registered.
#[derive(Resource, Debug, Default)]
pub struct ScrollbackState {
    scrollback: Scrollback,
    shared: SharedRows,
}

impl ScrollbackState {
    pub fn scrollback(&self) -> &Scrollback {
        &self.scrollback
    }
}

/// The scrollback's [`SidePanelProvider`].
pub struct ConversationPanelProvider {
    rows: SharedRows,
    /// The rows last shown and their cells.
    last: Mutex<Option<(Option<Vec<TextRow>>, PanelRows)>>,
}

impl ConversationPanelProvider {
    pub fn new(rows: SharedRows) -> Self {
        Self {
            rows,
            last: Mutex::new(None),
        }
    }
}

/// The panel's cells: the title, then the last [`PANEL_BODY_ROWS`] rows of
/// the conversation (none outside one).
pub fn scrollback_cells(rows: Option<&[TextRow]>) -> PanelRows {
    let rows = rows.unwrap_or(&[]);
    let shown = &rows[rows.len().saturating_sub(PANEL_BODY_ROWS)..];
    let glyphs: Vec<(&[u8], u16)> = shown
        .iter()
        .map(|row| (row.bytes.as_slice(), row.runic))
        .collect();
    PanelRows::with_glyph_rows(SCROLLBACK_TITLE, &glyphs)
}

impl SidePanelProvider for ConversationPanelProvider {
    fn kind(&self) -> PanelKind {
        PanelKind::Conversation
    }

    fn side(&self) -> SidePreference {
        SidePreference::Auto
    }

    /// `None` in the intro and title (no `PlayState`).
    fn snapshot(&self, state: Option<&PlayState>) -> Option<PanelContent> {
        state?;
        let rows = self
            .rows
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .clone();
        let mut last = self
            .last
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        match &*last {
            Some((shown, cells)) if *shown == rows => Some(PanelContent::Cells(cells.clone())),
            _ => {
                let cells = scrollback_cells(rows.as_deref());
                *last = Some((rows, cells.clone()));
                Some(PanelContent::Cells(cells))
            }
        }
    }
}

/// Adds the follow system, and the provider when `clue_scrollback` is on
/// at launch, when the side-space arbiter is in the app.
pub(crate) fn add_scrollback(app: &mut App, launch: &VisualLaunchConfig) {
    if !app.world().contains_resource::<PanelRegistry>() {
        return;
    }
    if launch.enhancements.toggles.on(SCROLLBACK) {
        register(app.world_mut());
    }
    app.add_systems(
        Update,
        follow_scrollback_toggle
            .run_if(resource_exists_and_changed::<Toggles>)
            .in_set(EnhSet::Apply),
    )
    .add_systems(
        Update,
        scrollback_accumulate
            .in_set(EnhSet::Observe)
            .run_if(resource_exists::<ScrollbackState>)
            .run_if(resource_exists::<VisualState>),
    );
}

fn register(world: &mut World) {
    let state = ScrollbackState::default();
    let Some(mut registry) = world.get_resource_mut::<PanelRegistry>() else {
        return;
    };
    registry.register(ConversationPanelProvider::new(state.shared.clone()));
    world.insert_resource(state);
}

/// Registers or removes the provider when `clue_scrollback` changes in
/// play (from the menu overlay).
fn follow_scrollback_toggle(
    mut commands: Commands,
    toggles: Res<Toggles>,
    state: Option<Res<ScrollbackState>>,
    mut registry: ResMut<PanelRegistry>,
) {
    match (toggles.0.on(SCROLLBACK), state.is_some()) {
        (true, false) => {
            let state = ScrollbackState::default();
            registry.register(ConversationPanelProvider::new(state.shared.clone()));
            commands.insert_resource(state);
        }
        (false, true) => {
            registry.remove(PanelKind::Conversation);
            commands.remove_resource::<ScrollbackState>();
        }
        _ => {}
    }
}

/// Steps the scrollback after the game's step and shares its rows.
pub(crate) fn scrollback_accumulate(mut state: ResMut<ScrollbackState>, visual: Res<VisualState>) {
    let state = &mut *state;
    state.scrollback.step(&visual.state);
    let rows = state.scrollback.rows();
    let mut shared = state
        .shared
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    if shared.as_deref() != rows {
        *shared = rows.map(<[TextRow]>::to_vec);
    }
}

#[cfg(test)]
mod tests;

//! The journal side panel (Phase B design 5.2.10, 5.3.5; build plan
//! M5-05).
//!
//! The journal owns no node, image or layout code for the panel: it
//! registers a [`JournalPanelProvider`] in the foundation's
//! [`PanelRegistry`] while `journal.side_panel` is on, and the foundation's
//! arbiter places it (`journal.side_panel_side`, `display.panel_priority`,
//! `display.panel_guard`) and its cell-panel renderer draws the rows in the
//! frame's own IBM.CH font. Under grow-left's world view the canvas fills
//! the window, so the arbiter hides the panel with its one-time toast; it
//! shows again on the fixed screens.
//!
//! **No spoilers.** The panel reads only what the player has achieved: the
//! virtue phases and shrine conditions, and counts and names of things
//! vanquished, opened or carried, each line only while above zero or
//! carried, with no denominator. That is never above tier 2, whatever tier
//! the page uses, and never a mantra.
//!
//! **The page.** While the Ctrl+J page is open the panel shows nothing (the
//! page already shows everything): [`copy_journal_page_open`], in
//! `EnhSet::Present` before the panel host, copies the overlay model's
//! state into the provider.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use bevy::prelude::*;
use u5_runtime::enhancements::{JournalPanelSide, Toggle};
use u5_runtime::quest_journal::{
    QuestSnapshot, ShrineCondition, ShrinePhase, StatusMark, VirtueSummary, text,
};
use u5_runtime::{PlayState, SPECIAL_ITEM_WOODEN_BOX_INDEX, ShrineVirtue, VIRTUE_COUNT};

use crate::enh::overlay::{OverlayId, OverlayState};
use crate::enh::side::{
    PANEL_BODY_ROWS, PANEL_COLUMNS, PanelContent, PanelKind, PanelRegistry, PanelRows,
    SidePanelProvider, SidePreference,
};
use crate::enh::{EnhLaunch, Toggles};

use super::wrap;

/// The panel's title row (5.2.10).
pub const PANEL_TITLE: &str = "QUEST";
/// The lower lines' labels; the count sits in column [`COUNT_COLUMN`].
pub const PANEL_VANQUISHED: &str = "Vanquished";
pub const PANEL_SEALS_OPEN: &str = "Seals open";
/// The marker column of a virtue row, and a count's column.
pub const MARK_COLUMN: usize = 14;
pub const COUNT_COLUMN: usize = 13;

/// What the 16x25 panel shows, derived from the play state, compared, then
/// turned into the host's [`PanelRows`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SidePanelModel {
    pub summary: VirtueSummary,
    pub virtues: [(ShrinePhase, ShrineCondition); VIRTUE_COUNT],
    pub shadowlords_vanquished: u8,
    pub seals_open: u8,
    /// One bit per carried regalia piece, in [`text::REGALIA`] order (the
    /// Crown, the Sceptre and the Amulet).
    pub regalia: u8,
    pub box_carried: bool,
}

impl SidePanelModel {
    pub fn from_snapshot(snap: &QuestSnapshot) -> Self {
        let virtues = ShrineVirtue::ALL.map(|virtue| (snap.phase(virtue), snap.condition(virtue)));
        let shadowlords_vanquished = snap
            .shadowlords
            .iter()
            .filter(|state| **state == u5_runtime::quest_journal::ShadowlordState::Vanquished)
            .count() as u8;
        let seals_open = (0..snap.seals.len())
            .filter(|record| snap.seal_open(*record))
            .count() as u8;
        let mut regalia = 0;
        for (bit, (_, index, _)) in regalia_pieces().enumerate() {
            if snap.special_items[*index] != 0 {
                regalia |= 1 << bit;
            }
        }
        Self {
            summary: snap.virtue_summary(),
            virtues,
            shadowlords_vanquished,
            seals_open,
            regalia,
            box_carried: snap.special_items[SPECIAL_ITEM_WOODEN_BOX_INDEX] != 0,
        }
    }

    pub fn from_play_state(state: &PlayState) -> Self {
        Self::from_snapshot(&QuestSnapshot::from_play_state(state))
    }

    /// The panel's lines below the title, at most [`PANEL_BODY_ROWS`].
    pub fn lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        let summary = self.summary;
        if summary == VirtueSummary::default() {
            lines.extend(wrap(text::NOTHING_RECORDED, PANEL_COLUMNS, 2));
        } else {
            lines.push(format!("{} done", summary.complete));
            lines.push(format!("{} in progress", summary.in_progress));
            if summary.ruined > 0 {
                lines.push(format!("{} ruined", summary.ruined));
            }
        }
        lines.push(String::new());
        for (virtue, (phase, condition)) in ShrineVirtue::ALL.iter().zip(self.virtues) {
            let mark = if condition == ShrineCondition::Ruined {
                StatusMark::ALERT
            } else {
                StatusMark::for_phase(phase)
            };
            // Column 15 stays blank, as in the 5.2.10 sketch; the page
            // shows a restored shrine's `r`.
            lines.push(format!(
                "{:<width$}{}",
                virtue.name(),
                mark.glyph,
                width = MARK_COLUMN
            ));
        }
        let mut found = Vec::new();
        for (label, count) in [
            (PANEL_VANQUISHED, self.shadowlords_vanquished),
            (PANEL_SEALS_OPEN, self.seals_open),
        ] {
            if count > 0 {
                found.push(format!("{label:<COUNT_COLUMN$}{count}"));
            }
        }
        for (bit, (label, _, _)) in regalia_pieces().enumerate() {
            if self.regalia & (1 << bit) != 0 {
                found.push((*label).to_string());
            }
        }
        if self.box_carried
            && let Some((label, _, _)) = text::REGALIA
                .iter()
                .find(|(_, index, _)| *index == SPECIAL_ITEM_WOODEN_BOX_INDEX)
        {
            found.push((*label).to_string());
        }
        if !found.is_empty() {
            lines.push(String::new());
            lines.extend(found);
        }
        lines.truncate(PANEL_BODY_ROWS);
        lines
    }

    pub fn rows(&self) -> PanelRows {
        let lines = self.lines();
        let lines: Vec<&str> = lines.iter().map(String::as_str).collect();
        PanelRows::new(PANEL_TITLE, &lines)
    }
}

/// The regalia proper: [`text::REGALIA`] without the Sandalwood Box.
fn regalia_pieces() -> impl Iterator<Item = &'static (&'static str, usize, &'static [&'static str])>
{
    text::REGALIA
        .iter()
        .filter(|(_, index, _)| *index != SPECIAL_ITEM_WOODEN_BOX_INDEX)
}

/// The side the provider asks for.
pub fn panel_side(side: JournalPanelSide) -> SidePreference {
    match side {
        JournalPanelSide::Auto => SidePreference::Auto,
        JournalPanelSide::Left => SidePreference::Left,
        JournalPanelSide::Right => SidePreference::Right,
    }
}

/// Whether the journal page is open, shared by the provider and
/// [`copy_journal_page_open`]. Present exactly while the provider is
/// registered.
#[derive(Resource, Clone, Debug, Default)]
pub struct JournalPanelPageOpen(pub Arc<AtomicBool>);

/// The journal's [`SidePanelProvider`] (`PanelKind::Journal`).
pub struct JournalPanelProvider {
    side: SidePreference,
    /// Written in `EnhSet::Present` before the host runs.
    page_open: Arc<AtomicBool>,
    /// The last model and its rows, so the rows are rebuilt only when the
    /// model changes.
    last: Mutex<Option<(SidePanelModel, PanelRows)>>,
}

impl JournalPanelProvider {
    pub fn new(side: JournalPanelSide, page_open: Arc<AtomicBool>) -> Self {
        Self {
            side: panel_side(side),
            page_open,
            last: Mutex::new(None),
        }
    }
}

impl SidePanelProvider for JournalPanelProvider {
    fn kind(&self) -> PanelKind {
        PanelKind::Journal
    }

    fn side(&self) -> SidePreference {
        self.side
    }

    /// `None` in the intro and title (no `PlayState`) and while the page
    /// is open.
    fn snapshot(&self, state: Option<&PlayState>) -> Option<PanelContent> {
        let state = state?;
        if self.page_open.load(Ordering::Relaxed) {
            return None;
        }
        let model = SidePanelModel::from_play_state(state);
        let mut last = self
            .last
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        match &*last {
            Some((shown, rows)) if *shown == model => Some(PanelContent::Cells(rows.clone())),
            _ => {
                let rows = model.rows();
                *last = Some((model, rows.clone()));
                Some(PanelContent::Cells(rows))
            }
        }
    }
}

/// Registers the provider and its page flag, when the foundation's arbiter
/// is in the app (it is whenever `journal.side_panel` was on at launch).
pub(crate) fn register_journal_panel(world: &mut World, side: JournalPanelSide) {
    let Some(mut registry) = world.get_resource_mut::<PanelRegistry>() else {
        return;
    };
    let flag = JournalPanelPageOpen::default();
    registry.register(JournalPanelProvider::new(side, flag.0.clone()));
    world.insert_resource(flag);
}

/// Copies `OverlayState::open == Some(OverlayId::Journal)` into the
/// provider, in `EnhSet::Present` before the panel host.
pub(crate) fn copy_journal_page_open(
    flag: Res<JournalPanelPageOpen>,
    overlay: Option<Res<OverlayState>>,
) {
    let open = overlay.is_some_and(|overlay| overlay.open() == Some(OverlayId::Journal));
    flag.0.store(open, Ordering::Relaxed);
}

/// Registers or removes the provider when `journal.side_panel` changes in
/// play (from the menu overlay).
pub(crate) fn follow_journal_panel_toggle(
    mut commands: Commands,
    toggles: Res<Toggles>,
    launch: Option<Res<EnhLaunch>>,
    flag: Option<Res<JournalPanelPageOpen>>,
    registry: Option<ResMut<PanelRegistry>>,
) {
    let Some(mut registry) = registry else {
        return;
    };
    match (toggles.0.on(Toggle::JournalSidePanel), flag.is_some()) {
        (true, false) => {
            let side = launch
                .map(|launch| launch.0.enhancements.journal().side)
                .unwrap_or_default();
            let flag = JournalPanelPageOpen::default();
            registry.register(JournalPanelProvider::new(side, flag.0.clone()));
            commands.insert_resource(flag);
        }
        (false, true) => {
            registry.remove(PanelKind::Journal);
            commands.remove_resource::<JournalPanelPageOpen>();
        }
        _ => {}
    }
}

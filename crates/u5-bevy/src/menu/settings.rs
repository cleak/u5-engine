//! The menu's settings plumbing (Phase B design 3.2.7, 3.3.6; build plan
//! M4-06, D-M4.4, D-M4.5).
//!
//! - [`settings_view`]: the Settings rows the menu shows. They come from the
//!   generator the setup wizard uses
//!   (`u5_runtime::enhancements::settings_rows`), fed the run's chosen
//!   values ([`LiveSettings::chosen`] with the changes still queued), so
//!   for the same resolution the menu and the wizard show the same rows.
//!   Each row has its [`RowNote`]: the effective apply point and the
//!   "(set by command line)" mark ([`note_words`]).
//! - [`menu_setting_commands`]: the reducer's `Flip` and `Step` on a
//!   setting row become a queued change ([`queue_setting_change`]), which
//!   the foundation's `apply_pending_changes` applies at the row's point
//!   and stores through the host's sink.
//!
//! M4-07 draws the pages from this view.

use bevy::prelude::*;
use u5_runtime::enhancements::page::{RowKind, RowValue, SettingRow, SettingsRows};
use u5_runtime::enhancements::{
    ApplyAt, FeatureToggles, LockState, ResolvedEnhancements, RowNote, SettingValue, apply_value,
    row_notes, settings_page_rows,
};

use super::MenuCommandEvent;
use super::pages::MenuCommand;
use super::text as t;
use crate::enh::{
    EnhLaunch, LiveSettings, PendingSettingChanges, PendingToggleChanges, queue_setting_change,
};

/// The Settings rows as the menu shows them now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingsView {
    /// The shared generator's rows and presets for the chosen values.
    pub page: SettingsRows,
    /// Each row's note, in row order.
    pub notes: Vec<RowNote>,
}

impl SettingsView {
    /// The row for `key` and its note.
    pub fn row(&self, key: &str) -> Option<(&SettingRow, RowNote)> {
        let index = self.page.rows.iter().position(|row| row.key == key)?;
        Some((&self.page.rows[index], self.notes[index]))
    }
}

/// The chosen values with the queued changes on top: what the rows show
/// while a change waits for its point.
pub fn chosen_with_queued(
    live: &LiveSettings,
    toggles: &PendingToggleChanges,
    settings: &PendingSettingChanges,
) -> ResolvedEnhancements {
    let mut chosen = live.chosen.clone();
    for (toggle, on) in &toggles.0 {
        chosen.toggles.set(*toggle, *on);
    }
    for (key, value) in &settings.0 {
        apply_value(&mut chosen, key, value.clone());
    }
    chosen
}

/// The Settings rows for a run launched with the toggles `launch`
/// (D-M4.5): the shared rows of the chosen values, each with its note. The
/// command-line marks are the launch's.
pub fn settings_view(
    live: &LiveSettings,
    launch: FeatureToggles,
    toggles: &PendingToggleChanges,
    settings: &PendingSettingChanges,
) -> SettingsView {
    let chosen = chosen_with_queued(live, toggles, settings);
    let page = settings_page_rows(&chosen, chosen.lock != LockState::Off, Vec::new());
    let notes = row_notes(&page.rows, &live.now, launch);
    SettingsView { page, notes }
}

/// The words a row shows after its value (3.2.7, 3.3.6): "next prompt" or
/// "next launch" when it does not apply on the next frame, then "(set by
/// command line)".
pub fn note_words(note: RowNote) -> Vec<&'static str> {
    let apply = match note.apply {
        ApplyAt::Frame => None,
        ApplyAt::IdlePrompt => Some(t::NEXT_PROMPT),
        ApplyAt::NextLaunch => Some(t::NEXT_LAUNCH),
    };
    apply
        .into_iter()
        .chain(note.by_cli.then_some(t::SET_BY_COMMAND_LINE))
        .collect()
}

/// The value a row takes for `Flip` (`step` 0) or a step left (-1) or
/// right (1): a switch flips (a step sets it on to the right, off to the
/// left); a choice cycles in its listed order on `Flip`, and a step moves
/// one place, by number for a number row (left is lower), stopping at the
/// ends as the cursor does. `None` when nothing changes.
pub fn stepped(row: &SettingRow, step: i8) -> Option<RowValue> {
    let next = match (&row.kind, &row.value) {
        (RowKind::Switch, RowValue::Switch(on)) => RowValue::Switch(match step {
            0 => !on,
            step => step > 0,
        }),
        (RowKind::Choice(choices), RowValue::Choice(value)) => {
            let mut order: Vec<&str> = choices.iter().map(|(choice, _)| choice.as_str()).collect();
            if step != 0 {
                let numbers: Option<Vec<i64>> =
                    order.iter().map(|choice| choice.parse().ok()).collect();
                if let Some(numbers) = numbers {
                    let mut paired: Vec<(i64, &str)> = numbers.into_iter().zip(order).collect();
                    paired.sort_by_key(|(number, _)| *number);
                    order = paired.into_iter().map(|(_, choice)| choice).collect();
                }
            }
            let at = order.iter().position(|choice| choice == value)?;
            let to = match step {
                0 => (at + 1) % order.len(),
                step if step < 0 => at.saturating_sub(1),
                _ => (at + 1).min(order.len() - 1),
            };
            RowValue::Choice(order[to].to_string())
        }
        _ => return None,
    };
    (next != row.value).then_some(next)
}

/// The value to queue for the reducer's `command`, if it is a change of a
/// setting row in `view`.
pub fn command_change(
    view: &SettingsView,
    command: MenuCommand,
) -> Option<(&'static str, SettingValue)> {
    let (key, step) = match command {
        MenuCommand::Flip(key) => (key, 0),
        MenuCommand::Step(key, step) => (key, step),
        _ => return None,
    };
    let (row, _) = view.row(key)?;
    let value = stepped(row, step)?;
    Some((key, value.to_setting(key)))
}

/// Carries out the reducer's `Flip` and `Step` on setting rows: each
/// becomes a queued change, applied by the foundation at the row's point.
pub(crate) fn menu_setting_commands(
    mut commands: EventReader<MenuCommandEvent>,
    live: Res<LiveSettings>,
    launch: Res<EnhLaunch>,
    mut toggles: ResMut<PendingToggleChanges>,
    mut settings: ResMut<PendingSettingChanges>,
) {
    for MenuCommandEvent(command) in commands.read() {
        if !matches!(command, MenuCommand::Flip(_) | MenuCommand::Step(..)) {
            continue;
        }
        let view = settings_view(&live, launch.0.enhancements.toggles, &toggles, &settings);
        if let Some((key, value)) = command_change(&view, *command) {
            queue_setting_change(key, value, &mut toggles, &mut settings);
        }
    }
}

#[cfg(test)]
#[path = "settings_tests.rs"]
mod tests;

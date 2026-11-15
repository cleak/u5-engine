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
//! - [`settings_tabs`] (M4-07): the four tabs' rows from that view (3.2.7):
//!   - Audio: Volume (a slider in steps of 5), Mute and Mute on focus
//!     loss;
//!   - Display: every `[display]` row the generator lists, in its order,
//!     each sub-option indented under its parent (the grow-left rows under
//!     "Wider world view"), chapter 9's rows among them, and no Tile
//!     graphics row (the generator leaves `display.raster` out);
//!   - Game ([`game_tab`], M4-13): Sound and Buffer with the game's live
//!     values, which type the game's own key, the start preferences and
//!     the unsaved warning, and Pacing, reserved;
//!   - Enhancements: the preset row (the preset in force, or Custom), then
//!     every row in its group under the group's heading; the Cheats group
//!     only while the catalogue has cheat rows and the lock is off
//!     (D-M4.9).
//!
//!   Labels come from the wording table ([`t::setting_label`]); the
//!   catalogue's longer label, or its help line, is the footer's help. A
//!   spoiler row ([`is_spoiler_row`]) starts its label and help with
//!   "Spoiler:".
//! - [`menu_setting_commands`] also carries out a preset choice: every row
//!   the preset governs takes its value, each queued as a change.

use bevy::prelude::*;
use u5_runtime::enhancements::page::{
    MixLabel, RowKind, RowValue, SettingRow, SettingsPage, SettingsRows,
};
use u5_runtime::enhancements::{
    ApplyAt, CatalogueRow, FeatureToggles, LockState, OptionKind, Phase, ResolvedEnhancements,
    RowNote, SETTING_GROUPS, SettingValue, Tag, apply_value, help_line, row_notes,
    settings_page_rows_in,
};

use u5_runtime::enhancements::menu::{
    CONFIRM_UNSAVED_KEY, START_BUFFER_ON_KEY, START_SOUND_OFF_KEY,
};

use super::MenuCommandEvent;
use super::pages::{GameKey, MenuCommand, Row, RowAction, RowKind as MenuRowKind, Shown};
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
    settings_view_in(
        live,
        launch,
        toggles,
        settings,
        u5_runtime::enhancements::SHIPPED_PHASES,
    )
}

/// [`settings_view`] for a build that has shipped the phases `shipped`
/// ([`super::MenuShell::shipped`]).
pub fn settings_view_in(
    live: &LiveSettings,
    launch: FeatureToggles,
    toggles: &PendingToggleChanges,
    settings: &PendingSettingChanges,
    shipped: &[Phase],
) -> SettingsView {
    let chosen = chosen_with_queued(live, toggles, settings);
    let page = settings_page_rows_in(&chosen, chosen.lock != LockState::Off, Vec::new(), shipped);
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
    if let (Some(range), RowValue::Choice(value), true) =
        (int_range(&row.key), &row.value, step != 0)
    {
        let next = number_step(range, value.parse().ok()?, step);
        let next = RowValue::Choice(next.to_string());
        return (next != row.value).then_some(next);
    }
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

/// The smallest step a number row takes in the menu (3.2.7: the volume in
/// steps of 5, the brightness slider in 5% steps).
pub const NUMBER_STEP: i64 = 5;

/// The range and step of the number setting `key`; `None` for any other
/// key.
pub fn int_range(key: &str) -> Option<(i64, i64, i64)> {
    match CatalogueRow::find(key)? {
        CatalogueRow::Option(spec) => match spec.kind {
            OptionKind::Int { min, max, step } => Some((min, max, step)),
            _ => None,
        },
        CatalogueRow::Toggle(_) => None,
    }
}

/// A number row's value one step down (`step` < 0) or up from `value`:
/// to the next multiple of the step ([`NUMBER_STEP`] or the catalogue's,
/// whichever is larger), within the row's range. A value between steps
/// goes to the step beside it.
pub fn number_step((min, max, step): (i64, i64, i64), value: i64, dir: i8) -> i64 {
    let s = step.max(NUMBER_STEP);
    let next = if dir > 0 {
        (value.div_euclid(s) + 1) * s
    } else if value.rem_euclid(s) != 0 {
        value.div_euclid(s) * s
    } else {
        value - s
    };
    next.clamp(min, max)
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
    shell: Res<super::MenuShell>,
    live: Res<LiveSettings>,
    launch: Res<EnhLaunch>,
    mut toggles: ResMut<PendingToggleChanges>,
    mut settings: ResMut<PendingSettingChanges>,
) {
    for MenuCommandEvent(command) in commands.read() {
        if !matches!(
            command,
            MenuCommand::Flip(_) | MenuCommand::Step(..) | MenuCommand::Preset(_)
        ) {
            continue;
        }
        let view = settings_view_in(
            &live,
            launch.0.enhancements.toggles,
            &toggles,
            &settings,
            shell.shipped,
        );
        for (key, value) in command_changes(&view, *command) {
            queue_setting_change(key, value, &mut toggles, &mut settings);
        }
    }
}

/// Every change the reducer's `command` makes on the rows of `view`: a
/// setting row's one ([`command_change`]), or a preset's, which gives every
/// row it governs its value (the rows already there are left alone).
pub fn command_changes(
    view: &SettingsView,
    command: MenuCommand,
) -> Vec<(&'static str, SettingValue)> {
    let MenuCommand::Preset(index) = command else {
        return command_change(view, command).into_iter().collect();
    };
    let Some(preset) = view.page.presets.get(usize::from(index)) else {
        return Vec::new();
    };
    preset
        .values
        .iter()
        .filter_map(|(key, value)| {
            let (row, _) = view.row(key)?;
            let key = CatalogueRow::find(key)?.key();
            (row.value != *value).then(|| (key, value.to_setting(key)))
        })
        .collect()
}

/// The game's own values the Game tab shows (3.2.7): Sound and Buffer as
/// the game holds them now.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct GameValues {
    pub sound_on: bool,
    pub buffer_on: bool,
}

/// The Audio tab's rows, in order (3.2.7).
pub const AUDIO_KEYS: [&str; 3] = ["audio.volume", "audio.muted", "audio.mute_on_focus_loss"];

/// Cells a sub-option is set in under its parent.
pub const SUB_INDENT: u8 = 2;
/// Cells a row is set in under its group's heading on the Extras tab.
pub const GROUP_INDENT: u8 = 1;

/// Whether the row `key` may show what the player has not found (U17,
/// 3.2.7): a `Tag::SpoilerRisk` row, or one of [`t::SPOILER_KEYS`].
pub fn is_spoiler_row(key: &str) -> bool {
    t::SPOILER_KEYS.contains(&key)
        || CatalogueRow::find(key).is_some_and(|row| row.tags().contains(&Tag::SpoilerRisk))
}

/// How a row's value shows: a switch as a check box, a number as a percent
/// with its slider filled over the row's range, a word as itself.
fn row_kind(key: &str, value: &RowValue) -> MenuRowKind {
    match value {
        RowValue::Switch(on) => MenuRowKind::Check { on: *on },
        RowValue::Choice(text) => MenuRowKind::Value(shown(key, text)),
    }
}

fn shown(key: &str, text: &str) -> Shown {
    match CatalogueRow::find(key) {
        Some(CatalogueRow::Option(spec)) => match spec.kind {
            OptionKind::Int { min, max, .. } => text.parse::<i64>().map_or(Shown::Blank, |v| {
                let fill = if max > min {
                    ((v - min) * 100 / (max - min)).clamp(0, 100)
                } else {
                    100
                };
                Shown::Percent {
                    value: v.clamp(0, 255) as u8,
                    fill: fill as u8,
                }
            }),
            OptionKind::Enum(words) => words
                .iter()
                .find(|word| **word == text)
                .map_or(Shown::Blank, |word| Shown::Word(word)),
            _ => Shown::Blank,
        },
        _ => Shown::Blank,
    }
}

/// The menu's row for the generator's `row` with its `note`, set in by
/// `indent` cells; `None` for a key the catalogue does not have.
pub fn setting_row(row: &SettingRow, note: RowNote, indent: u8) -> Option<Row> {
    let entry = CatalogueRow::find(&row.key)?;
    let key = entry.key();
    let label = t::setting_label(key).unwrap_or(entry.label());
    let help = t::setting_help(key)
        .or_else(|| help_line(key))
        .or((entry.label() != label).then_some(entry.label()));
    let tag = match note.apply {
        ApplyAt::Frame => None,
        ApplyAt::IdlePrompt => Some(t::NEXT_PROMPT),
        ApplyAt::NextLaunch => Some(t::NEXT_LAUNCH),
    };
    let mut out = Row {
        tag,
        by_cli: note.by_cli,
        ..Row::item(label, RowAction::Setting(key))
            .indented(indent)
            .with_help(help)
    };
    out.kind = row_kind(key, &row.value);
    Some(out)
}

/// The static name of the generator's group `name`.
fn group_label(name: &str) -> &'static str {
    SETTING_GROUPS
        .iter()
        .find(|(group, _)| *group == name)
        .map_or("", |(group, _)| group)
}

/// The Game tab's settings rows after Sound and Buffer, in order (3.2.7).
pub const GAME_KEYS: [&str; 3] = [
    START_SOUND_OFF_KEY,
    START_BUFFER_ON_KEY,
    CONFIRM_UNSAVED_KEY,
];

/// The Game tab (3.2.7 "Game"):
///
/// - Sound and Buffer with the game's live values; Enter closes the menu
///   and types Ctrl-S or Ctrl-B ([`RowAction::GameKey`]). Before play
///   (the title layer) there is no game to type to, and they are greyed.
/// - The two start preferences and the unsaved warning, the generator's
///   rows like every other setting (a row the generator does not list is
///   left out).
/// - Pacing, reserved and greyed.
pub fn game_tab(view: &SettingsView, game: Option<GameValues>) -> Vec<Row> {
    let word = |on: bool| Shown::Word(if on { t::ON } else { t::OFF });
    let live = |label, key, on: Option<bool>| {
        let row = Row {
            action: RowAction::GameKey(key),
            ..Row::shown(label, "", on.map_or(Shown::Blank, word))
        };
        let help = match key {
            GameKey::Sound => t::SOUND_HELP,
            GameKey::Buffer => t::BUFFER_HELP,
        };
        match on {
            Some(_) => row.with_help(Some(help)),
            None => row.greyed(t::GAME_IN_PLAY),
        }
    };
    let mut rows = vec![
        live(t::SOUND, GameKey::Sound, game.map(|g| g.sound_on)),
        live(t::BUFFER, GameKey::Buffer, game.map(|g| g.buffer_on)),
    ];
    rows.extend(
        GAME_KEYS
            .iter()
            .filter_map(|key| view.row(key))
            .filter_map(|(row, note)| setting_row(row, note, 0)),
    );
    rows.push(Row {
        action: RowAction::None,
        ..Row::shown(t::PACING, "", Shown::Word(t::RESERVED)).greyed(t::PACING_LATER)
    });
    rows
}

/// The rows of the four tabs, by [`super::pages::SettingsTab::index`], for
/// the settings in `view` and the game's own values `game` (none before
/// play).
pub fn settings_tabs(view: &SettingsView, game: Option<GameValues>) -> [Vec<Row>; 4] {
    let rows = &view.page.rows;
    let notes = &view.notes;
    let ordinary = || {
        rows.iter()
            .zip(notes)
            .filter(|(row, _)| !row.role.is_cheat_group())
    };
    let sub = |row: &SettingRow| if row.indent { SUB_INDENT } else { 0 };

    let audio = AUDIO_KEYS
        .iter()
        .filter_map(|key| view.row(key))
        .filter_map(|(row, note)| setting_row(row, note, 0))
        .collect();

    let display = ordinary()
        .filter(|(row, _)| row.key.starts_with("display."))
        .filter_map(|(row, note)| setting_row(row, *note, sub(row)))
        .collect();

    let game_rows = game_tab(view, game);

    let page = SettingsPage::new(view.page.clone());
    let current = match page.mix() {
        MixLabel::Preset(at) => Some(at as u8),
        MixLabel::Custom(_) => None,
    };
    let mut extras =
        vec![Row::preset(current, view.page.presets.len() as u8).with_help(Some(t::PRESET_HELP))];
    let mut group = None;
    for (row, note) in ordinary() {
        if group != Some(row.group.as_str()) {
            group = Some(row.group.as_str());
            extras.push(Row::header(group_label(&row.group)));
        }
        extras.extend(setting_row(row, *note, GROUP_INDENT + sub(row)));
    }
    if page.shows_cheats() {
        extras.push(Row::item(t::CHEATS_GROUP, RowAction::None).greyed(t::CHEATS_GROUP_HELP));
    }
    [audio, display, game_rows, extras]
}

#[cfg(test)]
#[path = "settings_tests.rs"]
mod tests;

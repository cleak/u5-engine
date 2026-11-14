//! The menu's pages, its model and its reducer (Phase B design 3.2.1,
//! 3.3.4 "Pages"; build plan M4-04).
//!
//! Everything here is pure and needs no window:
//!
//! - a page is a function from the shell's state ([`ShellSnapshot`]) to
//!   its rows ([`page_rows`]), each with an enabled flag and, when greyed,
//!   the reason;
//! - the model ([`MenuModel`]) is a stack of pages, each with its cursor;
//! - the reducer ([`reduce`]) turns one key ([`MenuInput`]) into model
//!   changes and the [`MenuCommand`]s the plugin carries out.
//!
//! Keys (3.2.1): F12 closes from any page; Escape goes back one page, or
//! closes from the top page; Up and Down move the cursor (stopping at the
//! ends); Enter selects; Space flips a check box; Left and Right change a
//! value, and on the Settings page switch its tab when the row has no
//! value; Tab opens the full map only while its door is in the menu
//! (D-M4.9). A greyed row can be selected, so its reason shows, but gives
//! no command. A greyed value row cannot change, so Left and Right switch
//! the tab on it. Group headings ([`RowKind::Header`]) are never selected:
//! the cursor passes over them ([`settle`]). On the Extras tab's preset row
//! Left and Right choose the preset before or after the one in force
//! ([`MenuCommand::Preset`]). Text ([`MenuInput::Text`]) is for the slot
//! name field (M4-10); no page here reads it.
//!
//! The rows of the Settings tabs come from the shell snapshot, which the
//! plugin fills from the catalogue (`settings::settings_tabs`, M4-07).

use bevy::prelude::*;

use super::text as t;
use crate::enh::input::{OverlayKey, OverlayText};
use crate::enh::overlay::OverlayId;

/// A top-menu item that opens another overlay (3.3.4). The owner's plugin
/// pushes it into [`MenuDoors`], so it is listed only while that feature
/// is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MenuDoor {
    /// The item's label, from the owner's text module.
    pub label: &'static str,
    /// The overlay it opens; the menu closes as that one opens.
    pub opens: OverlayId,
}

/// The doors of the top menu. Ships empty: the journal (M5), the full map
/// (chapter 4) and the cheats page (M8) do not exist yet (D-M4.9).
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct MenuDoors(pub Vec<MenuDoor>);

/// The four Settings tabs, in order (3.2.7).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SettingsTab {
    Audio,
    Display,
    Game,
    Enhancements,
}

impl SettingsTab {
    pub const ALL: [SettingsTab; 4] = [
        SettingsTab::Audio,
        SettingsTab::Display,
        SettingsTab::Game,
        SettingsTab::Enhancements,
    ];

    /// Its place in [`SettingsTab::ALL`].
    pub fn index(self) -> usize {
        self as usize
    }

    /// The tab's label on the strip.
    pub fn label(self) -> &'static str {
        t::SETTINGS_TABS[self.index()]
    }

    /// The next tab to the right (`step` 1) or left (`step` -1), stopping
    /// at the ends as the cursor does.
    fn step(self, step: i8) -> SettingsTab {
        let at = self.index() as i32 + i32::from(step);
        SettingsTab::ALL[at.clamp(0, SettingsTab::ALL.len() as i32 - 1) as usize]
    }
}

/// A page of the menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Page {
    /// The top menu (3.2.1).
    Top,
    /// Settings, on one of its tabs (3.2.7).
    Settings(SettingsTab),
}

/// What a row does when chosen.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RowAction {
    /// Close the menu.
    Resume,
    /// Open the Save page (M4-10).
    Save,
    /// Open the Load page (M4-11).
    Load,
    /// Close the menu and open another overlay.
    Door(OverlayId),
    /// Open the Settings page.
    Settings,
    /// Restart at the title (M4-13).
    QuitToTitle,
    /// Close the game (M4-07).
    QuitToDesktop,
    /// A setting, by its catalogue key: a check box flips it, a value
    /// steps it.
    Setting(&'static str),
    /// The Enhancements tab's preset row: Left and Right choose a preset.
    Preset,
    /// Nothing: a group heading, or a row shown only for its value.
    None,
}

/// How a value row shows its value (3.2.7).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Shown {
    /// No value drawn.
    Blank,
    /// A word: a choice's value ("fit"), or the game's own "On" and "Off".
    Word(&'static str),
    /// A percent with its slider, filled to `fill` percent of the row's
    /// range (`[####----]  70%`).
    Percent { value: u8, fill: u8 },
}

/// How a row shows its state and which keys it takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RowKind {
    /// A plain item: Enter chooses it.
    Item,
    /// A check box: Space or Enter flips it.
    Check { on: bool },
    /// A value: Left and Right step it.
    Value(Shown),
    /// A group heading on the Enhancements tab: never selected; the
    /// cursor passes over it.
    Header,
    /// The preset row (3.2.7): the presets, the one in force (`None` for
    /// Custom). Left and Right choose the one before or after it.
    Preset { current: Option<u8>, count: u8 },
}

/// One row of a page.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Row {
    pub label: &'static str,
    pub action: RowAction,
    pub kind: RowKind,
    /// A greyed row gives no command.
    pub enabled: bool,
    /// Why a greyed row is greyed, shown in its help line.
    pub reason: Option<&'static str>,
    /// Cells the label is set in from the margin: a group's rows and a
    /// sub-option under its parent (3.2.7).
    pub indent: u8,
    /// The apply point when it is not the next frame: "next prompt" or
    /// "next launch" (3.4.1, D-M4.4).
    pub tag: Option<&'static str>,
    /// The command line set this row for this run (3.3.6).
    pub by_cli: bool,
    /// The sentence the footer shows while the row is selected.
    pub help: Option<&'static str>,
}

impl Row {
    const fn new(label: &'static str, action: RowAction, kind: RowKind) -> Self {
        Self {
            label,
            action,
            kind,
            enabled: true,
            reason: None,
            indent: 0,
            tag: None,
            by_cli: false,
            help: None,
        }
    }

    /// An enabled item.
    pub const fn item(label: &'static str, action: RowAction) -> Self {
        Self::new(label, action, RowKind::Item)
    }

    /// An enabled check box for the setting `key`.
    pub const fn check(label: &'static str, key: &'static str, on: bool) -> Self {
        Self::new(label, RowAction::Setting(key), RowKind::Check { on })
    }

    /// An enabled value row for the setting `key`, its value not shown.
    pub const fn value(label: &'static str, key: &'static str) -> Self {
        Self::shown(label, key, Shown::Blank)
    }

    /// An enabled value row for the setting `key` showing `shown`.
    pub const fn shown(label: &'static str, key: &'static str, shown: Shown) -> Self {
        Self::new(label, RowAction::Setting(key), RowKind::Value(shown))
    }

    /// A group heading.
    pub const fn header(label: &'static str) -> Self {
        Self::new(label, RowAction::None, RowKind::Header)
    }

    /// The preset row: `count` presets, `current` in force (`None`:
    /// Custom).
    pub const fn preset(current: Option<u8>, count: u8) -> Self {
        Self::new(
            super::text::PRESET,
            RowAction::Preset,
            RowKind::Preset { current, count },
        )
    }

    /// The row greyed for `reason`.
    pub const fn greyed(mut self, reason: &'static str) -> Self {
        self.enabled = false;
        self.reason = Some(reason);
        self
    }

    /// The row set in by `cells`.
    pub const fn indented(mut self, cells: u8) -> Self {
        self.indent = cells;
        self
    }

    /// The row with its footer sentence.
    pub const fn with_help(mut self, help: Option<&'static str>) -> Self {
        self.help = help;
        self
    }

    /// Whether the cursor can rest on the row: anything but a heading.
    pub const fn selectable(&self) -> bool {
        !matches!(self.kind, RowKind::Header)
    }
}

/// Where the cursor rests on `rows` when it was asked for `cursor`: that
/// row, or the next row it can rest on below it, or above it; 0 for a
/// page with none.
pub fn settle(rows: &[Row], cursor: usize) -> usize {
    let cursor = cursor.min(rows.len().saturating_sub(1));
    (cursor..rows.len())
        .find(|i| rows[*i].selectable())
        .or_else(|| (0..cursor).rev().find(|i| rows[*i].selectable()))
        .unwrap_or(cursor)
}

/// The next row the cursor can rest on from `cursor` going down (`step`
/// 1) or up (-1), stopping at the ends.
fn step_cursor(rows: &[Row], cursor: usize, step: i8) -> usize {
    if step > 0 {
        (cursor + 1..rows.len())
            .find(|i| rows[*i].selectable())
            .unwrap_or(cursor)
    } else {
        (0..cursor)
            .rev()
            .find(|i| rows[*i].selectable())
            .unwrap_or(cursor)
    }
}

/// The shell's state the pages read: a snapshot taken when a key arrives.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ShellSnapshot {
    /// The default launch (the intro, then play) rather than the
    /// `--visual` harness shell, which has no intro and offers only Resume,
    /// Settings and Leave the game (3.2.1).
    pub intro_shell: bool,
    /// Named slots are on and the host keeps a slot store: Save and Load
    /// are listed.
    pub slots: bool,
    /// `Q` would reach the game's save prompt here
    /// (`save_command_available`, M4-10). Save is greyed otherwise.
    pub save_available: bool,
    /// The doors the features in force have pushed.
    pub doors: Vec<MenuDoor>,
    /// The rows of each Settings tab, by [`SettingsTab::index`] (M4-06 and
    /// M4-07 fill them).
    pub settings: [Vec<Row>; 4],
}

/// The rows of `page` for the shell in `snap`.
pub fn page_rows(page: Page, snap: &ShellSnapshot) -> Vec<Row> {
    match page {
        Page::Top => top_rows(snap),
        Page::Settings(tab) => snap.settings[tab.index()].clone(),
    }
}

/// The top menu (3.2.1): each item only while its feature is on.
fn top_rows(snap: &ShellSnapshot) -> Vec<Row> {
    let mut rows = vec![Row::item(t::RESUME, RowAction::Resume)];
    if !snap.intro_shell {
        // The `--visual` harness shell's short menu.
        rows.push(Row::item(t::SETTINGS, RowAction::Settings));
        rows.push(Row::item(t::QUIT_TO_DESKTOP, RowAction::QuitToDesktop));
        return rows;
    }
    if snap.slots {
        let save = Row::item(t::SAVE, RowAction::Save);
        rows.push(if snap.save_available {
            save
        } else {
            save.greyed(t::SAVE_UNAVAILABLE)
        });
        rows.push(Row::item(t::LOAD, RowAction::Load));
    }
    let door = |door: &MenuDoor| Row::item(door.label, RowAction::Door(door.opens));
    let is_cheats = |door: &&MenuDoor| door.opens == OverlayId::Cheats;
    rows.extend(snap.doors.iter().filter(|d| !is_cheats(d)).map(door));
    rows.push(Row::item(t::SETTINGS, RowAction::Settings));
    rows.extend(snap.doors.iter().filter(is_cheats).map(door));
    rows.push(Row::item(t::QUIT_TO_TITLE, RowAction::QuitToTitle));
    rows.push(Row::item(t::QUIT_TO_DESKTOP, RowAction::QuitToDesktop));
    rows
}

/// One page on the stack, with its cursor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PageState {
    pub page: Page,
    /// The selected row. Kept within the page's rows when a key arrives.
    pub cursor: usize,
}

/// The open menu: a stack of pages, the top menu at the bottom. Exists
/// only while the menu is open.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuModel {
    stack: Vec<PageState>,
}

impl Default for MenuModel {
    fn default() -> Self {
        Self::new()
    }
}

impl MenuModel {
    /// The menu as it opens: the top page, the first row selected.
    pub fn new() -> Self {
        Self {
            stack: vec![PageState {
                page: Page::Top,
                cursor: 0,
            }],
        }
    }

    /// The page showing.
    pub fn current(&self) -> PageState {
        *self.stack.last().expect("the top page is never popped")
    }

    /// Every page on the stack, the top menu first.
    pub fn stack(&self) -> &[PageState] {
        &self.stack
    }

    fn top_mut(&mut self) -> &mut PageState {
        self.stack.last_mut().expect("the top page is never popped")
    }

    fn push(&mut self, page: Page) {
        self.stack.push(PageState { page, cursor: 0 });
    }

    /// Goes back one page; false on the top page.
    fn pop(&mut self) -> bool {
        if self.stack.len() > 1 {
            self.stack.pop();
            true
        } else {
            false
        }
    }
}

/// One input to the menu, from the router or the QA script.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuInput {
    Key(OverlayKey),
    Text(char),
}

impl From<OverlayKey> for MenuInput {
    fn from(key: OverlayKey) -> Self {
        MenuInput::Key(key)
    }
}

impl From<OverlayText> for MenuInput {
    fn from(text: OverlayText) -> Self {
        MenuInput::Text(text.0)
    }
}

/// What the menu asks the shell to do. Page changes stay in the model.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuCommand {
    /// Close the menu.
    Close,
    /// Close the menu and open this overlay in its place.
    OpenDoor(OverlayId),
    /// Open the Save page (M4-10).
    OpenSave,
    /// Open the Load page (M4-11).
    OpenLoad,
    /// Restart at the title (M4-13).
    QuitToTitle,
    /// Close the game (M4-07).
    QuitToDesktop,
    /// Flip the check box of the setting with this catalogue key.
    Flip(&'static str),
    /// Step the setting with this catalogue key down (-1) or up (1).
    Step(&'static str, i8),
    /// Choose the preset at this place of the preset row (3.2.7): every
    /// row it governs takes its value.
    Preset(u8),
}

/// Applies `input` to `model` for the shell in `snap` and returns the
/// commands it gives (at most one). See the module notes for the keys.
pub fn reduce(model: &mut MenuModel, input: MenuInput, snap: &ShellSnapshot) -> Vec<MenuCommand> {
    let MenuInput::Key(key) = input else {
        return Vec::new();
    };
    let state = model.current();
    let rows = page_rows(state.page, snap);
    let cursor = settle(&rows, state.cursor);
    model.top_mut().cursor = cursor;
    let selected = rows
        .get(cursor)
        .filter(|row| row.enabled && row.selectable());
    let command = match key.key {
        KeyCode::F12 => Some(MenuCommand::Close),
        KeyCode::Escape => (!model.pop()).then_some(MenuCommand::Close),
        KeyCode::ArrowUp => {
            model.top_mut().cursor = step_cursor(&rows, cursor, -1);
            None
        }
        KeyCode::ArrowDown => {
            model.top_mut().cursor = step_cursor(&rows, cursor, 1);
            None
        }
        KeyCode::Enter | KeyCode::NumpadEnter => selected.and_then(|row| choose(model, row)),
        KeyCode::Space => selected.and_then(|row| match (row.kind, row.action) {
            (RowKind::Check { .. }, RowAction::Setting(key)) => Some(MenuCommand::Flip(key)),
            _ => None,
        }),
        KeyCode::ArrowLeft | KeyCode::ArrowRight => {
            let step = if key.key == KeyCode::ArrowLeft { -1 } else { 1 };
            // A greyed value cannot change, so on it the keys switch the
            // tab as on a row without a value.
            let value = rows.get(cursor).filter(|row| {
                row.enabled && matches!(row.kind, RowKind::Value(_) | RowKind::Preset { .. })
            });
            match (value, state.page) {
                (Some(row), _) => match (row.enabled, row.action, row.kind) {
                    (true, RowAction::Setting(key), _) => Some(MenuCommand::Step(key, step)),
                    (true, RowAction::Preset, RowKind::Preset { current, count }) => {
                        preset_step(current, count, step).map(MenuCommand::Preset)
                    }
                    _ => None,
                },
                (None, Page::Settings(tab)) => {
                    let next = tab.step(step);
                    if next != tab {
                        *model.top_mut() = PageState {
                            page: Page::Settings(next),
                            cursor: 0,
                        };
                    }
                    None
                }
                (None, Page::Top) => None,
            }
        }
        KeyCode::Tab => snap
            .doors
            .iter()
            .any(|door| door.opens == OverlayId::Map)
            .then_some(MenuCommand::OpenDoor(OverlayId::Map)),
        _ => None,
    };
    command.into_iter().collect()
}

/// Enter on an enabled `row`: a page push or a command.
fn choose(model: &mut MenuModel, row: &Row) -> Option<MenuCommand> {
    match (row.action, row.kind) {
        (RowAction::Resume, _) => Some(MenuCommand::Close),
        (RowAction::Save, _) => Some(MenuCommand::OpenSave),
        (RowAction::Load, _) => Some(MenuCommand::OpenLoad),
        (RowAction::Door(id), _) => Some(MenuCommand::OpenDoor(id)),
        (RowAction::Settings, _) => {
            model.push(Page::Settings(SettingsTab::Audio));
            None
        }
        (RowAction::QuitToTitle, _) => Some(MenuCommand::QuitToTitle),
        (RowAction::QuitToDesktop, _) => Some(MenuCommand::QuitToDesktop),
        (RowAction::Setting(key), RowKind::Check { .. }) => Some(MenuCommand::Flip(key)),
        (RowAction::Setting(_) | RowAction::Preset | RowAction::None, _) => None,
    }
}

/// The preset Left (`step` -1) or Right (1) chooses on the preset row: the
/// one before or after `current`, stopping at the ends. From Custom, Left
/// chooses the last preset and Right the first, as if Custom sat past
/// both ends; `None` when nothing changes.
fn preset_step(current: Option<u8>, count: u8, step: i8) -> Option<u8> {
    if count == 0 {
        return None;
    }
    match current {
        None if step < 0 => Some(count - 1),
        None => Some(0),
        Some(at) if step < 0 => at.checked_sub(1),
        Some(at) => (at + 1 < count).then_some(at + 1),
    }
}

#[cfg(test)]
mod tests;

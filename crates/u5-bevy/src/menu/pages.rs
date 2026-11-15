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
//! name field only.
//!
//! The rows of the Settings tabs come from the shell snapshot, which the
//! plugin fills from the catalogue (`settings::settings_tabs`, M4-07).
//!
//! **The slot pages** (3.2.2, 3.2.3; plan M4-11) list the slots the host
//! keeps ([`ShellSnapshot::slot_list`]), which the slot plugin refreshes
//! when Save or Load opens and after each change:
//!
//! - Save: "A new slot..." (the name field), the quick slot while
//!   `menu.quicksave` is on (it never asks), then every slot that is not
//!   damaged (it asks before writing over it). Choosing one gives
//!   [`MenuCommand::SaveSlot`]; the slot plugin types the game's own `Q`
//!   and `Y`, and the menu closes.
//! - Load: every slot, damaged ones greyed. Enter on one that is not
//!   damaged shows "Loading..." and gives [`MenuCommand::LoadSlot`]; F2
//!   renames a slot (not the quick one); Delete asks, then deletes.
//! - The name field takes printable ASCII typed text only, up to
//!   [`t::SLOT_NAME_MAX`] characters; Backspace takes the last one off.
//!   Enter saves (an empty name becomes "Slot N"; a rename needs a name),
//!   Escape goes back.
//! - The dialogs' buttons are chosen with Left and Right, the second
//!   (No, Cancel) first; Enter presses the chosen one.
//!
//! Slots are named by their place in the snapshot's list; the plugin
//! checks the place still holds a slot before it acts.

use std::sync::Arc;

use bevy::prelude::*;

use super::host::SlotInfo;
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
    /// Save: where to save (3.2.2).
    Save,
    /// Load: the slots and the selected one's details (3.2.3).
    Load,
    /// A dialog over the page below it.
    Dialog(Dialog),
    /// The title layer (3.2.4), over the title menu before play.
    Title,
}

/// A dialog of the slot pages. A slot is named by its place in
/// [`ShellSnapshot::slot_list`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Dialog {
    /// The name field (3.2.2).
    Name(NameFor),
    /// "Write over ...?" before saving over a slot.
    Overwrite(u16),
    /// "Delete ...?" before deleting a slot.
    Delete(u16),
    /// "Loading...", while the host restores the slot (3.3.9).
    Loading(u16),
    /// "... will be lost.": asked before leaving play with unsaved
    /// progress (3.2.6).
    Unsaved(Leave),
}

/// What the unsaved-progress question guards (3.2.6).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Leave {
    /// Load the slot at this place of the list.
    Load(u16),
    /// Return to the title.
    Title,
    /// Leave the game.
    Desktop,
}

/// Play that leaving now would lose (3.2.6, 3.3.5 "Unsaved progress").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Unsaved {
    /// The turn counter moved this many turns (at least one) since the
    /// last save or load seen this session.
    Turns(u64),
    /// No save or load was seen this session.
    Session,
}

/// One of the game's own switches the Game tab types (3.2.7 "Game", 3.3.8
/// "Injection").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GameKey {
    /// Ctrl-S, Sound On and Off.
    Sound,
    /// Ctrl-B, Buffer On and Off.
    Buffer,
}

/// What the name field names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NameFor {
    /// A new slot.
    New,
    /// The slot at this place, renamed (F2).
    Rename(u16),
}

/// The second of a dialog's two buttons (No, Cancel): where its cursor
/// starts, so Enter at once never writes over or deletes anything.
pub const SECOND_BUTTON: usize = 1;

/// One listed slot: what the host said, and its thumbnail decoded.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SlotEntry {
    /// The host's listing, without the PNG bytes.
    pub info: SlotInfo,
    /// The 80x50 RGBA thumbnail, when the slot has one that decodes.
    pub thumb: Option<Vec<u8>>,
}

/// The slots as last listed, the quick slot first and then newest first.
/// Shared, so a snapshot copies a pointer.
pub type SlotList = Arc<Vec<SlotEntry>>;

/// A row of the Save page.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SaveChoice {
    /// "A new slot...": the name field.
    New,
    /// The quick slot: saves at once.
    Quick,
    /// The slot at this place in the list: asks first.
    Existing(u16),
}

/// The Save page's rows for `snap` (3.2.2): a new slot, the quick slot
/// while `menu.quicksave` is on, then every other slot that is not
/// damaged, in the list's order.
pub fn save_choices(snap: &ShellSnapshot) -> Vec<SaveChoice> {
    let mut rows = vec![SaveChoice::New];
    if snap.quicksave {
        rows.push(SaveChoice::Quick);
    }
    rows.extend(
        snap.slot_list
            .iter()
            .enumerate()
            .filter(|(_, slot)| !slot.info.quick && slot.info.damaged.is_none())
            .map(|(i, _)| SaveChoice::Existing(i as u16)),
    );
    rows
}

/// The quick slot as listed, if it exists.
pub fn quick_slot(snap: &ShellSnapshot) -> Option<&SlotEntry> {
    snap.slot_list.iter().find(|slot| slot.info.quick)
}

/// Whether `ch` may go in a slot name: printable ASCII (3.2.2).
pub fn name_char(ch: char) -> bool {
    (' '..='~').contains(&ch)
}

/// What a row does when chosen.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RowAction {
    /// Close the menu.
    Resume,
    /// The title layer's Continue (3.2.4): load the slot at this place of
    /// the list, or with `None` press the game's own Journey Onward on the
    /// working save.
    Continue(Option<u16>),
    /// Open the Save page.
    Save,
    /// Open the Load page.
    Load,
    /// Close the menu and open another overlay.
    Door(OverlayId),
    /// Open the Settings page.
    Settings,
    /// Restart at the title (M4-12).
    QuitToTitle,
    /// Close the game (M4-07).
    QuitToDesktop,
    /// A setting, by its catalogue key: a check box flips it, a value
    /// steps it.
    Setting(&'static str),
    /// The Enhancements tab's preset row: Left and Right choose a preset.
    Preset,
    /// The Game tab's Sound or Buffer row: Enter closes the menu and types
    /// the game's own key.
    GameKey(GameKey),
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
    /// `menu.quicksave`: the Save page offers the quick slot.
    pub quicksave: bool,
    /// The slots as the host last listed them (M4-11).
    pub slot_list: SlotList,
    /// Turns since the last save or load this session (3.3.5 "Unsaved
    /// progress"); `None` before the first.
    pub turns_since_save: Option<u64>,
    /// What Load, Return to the title and Leave the game would lose, while
    /// `menu.confirm_unsaved` is on and the game is in play; `None` when
    /// they need not ask (3.2.6).
    pub unsaved: Option<Unsaved>,
}

/// The rows of `page` for the shell in `snap`.
pub fn page_rows(page: Page, snap: &ShellSnapshot) -> Vec<Row> {
    match page {
        Page::Top => top_rows(snap),
        Page::Title => title_rows(snap),
        Page::Settings(tab) => snap.settings[tab.index()].clone(),
        // The slot pages draw their own lines from the slot list.
        Page::Save | Page::Load | Page::Dialog(_) => Vec::new(),
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

/// The title layer (3.2.4). With slots: Continue (the newest slot that
/// can be loaded), Load, Settings and Quit; without: Continue, which
/// presses the game's own Journey Onward on the working save, Settings and
/// Quit. With slots but none to load, Continue is the plain one.
fn title_rows(snap: &ShellSnapshot) -> Vec<Row> {
    let mut rows = vec![Row::item(
        t::CONTINUE,
        RowAction::Continue(snap.slots.then(|| newest_slot(snap)).flatten()),
    )];
    if snap.slots {
        rows.push(Row::item(t::LOAD, RowAction::Load));
    }
    rows.push(Row::item(t::SETTINGS, RowAction::Settings));
    rows.push(Row::item(t::QUIT, RowAction::QuitToDesktop));
    rows
}

/// The place in the list of the newest slot that is not damaged: the
/// latest save time, the first listed on a tie.
pub fn newest_slot(snap: &ShellSnapshot) -> Option<u16> {
    snap.slot_list
        .iter()
        .enumerate()
        .filter(|(_, slot)| slot.info.damaged.is_none())
        .fold(None::<(usize, u64)>, |best, (at, slot)| match best {
            Some((_, time)) if time >= slot.info.saved_at_unix => best,
            _ => Some((at, slot.info.saved_at_unix)),
        })
        .map(|(at, _)| at as u16)
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
    /// The name field's text: printable ASCII, at most
    /// [`t::SLOT_NAME_MAX`] characters.
    field: String,
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
            field: String::new(),
        }
    }

    /// The name field's text.
    pub fn field(&self) -> &str {
        &self.field
    }

    /// The page showing.
    pub fn current(&self) -> PageState {
        *self.stack.last().expect("the top page is never popped")
    }

    /// The title layer as it opens (3.2.4): its first row, Continue,
    /// selected.
    pub fn title() -> Self {
        Self {
            stack: vec![PageState {
                page: Page::Title,
                cursor: 0,
            }],
            field: String::new(),
        }
    }

    /// Whether this is the title layer rather than the menu in play.
    pub fn is_title(&self) -> bool {
        self.stack
            .first()
            .is_some_and(|base| base.page == Page::Title)
    }

    /// Every page on the stack, the top menu first.
    pub fn stack(&self) -> &[PageState] {
        &self.stack
    }

    fn top_mut(&mut self) -> &mut PageState {
        self.stack.last_mut().expect("the top page is never popped")
    }

    fn push(&mut self, page: Page) {
        self.push_at(page, 0);
    }

    fn push_at(&mut self, page: Page, cursor: usize) {
        self.stack.push(PageState { page, cursor });
    }

    /// Keeps the page's cursor within `len` rows (a list that shrank).
    pub(crate) fn clamp_cursor(&mut self, len: usize) {
        let top = self.top_mut();
        top.cursor = top.cursor.min(len.saturating_sub(1));
    }

    /// Goes back one page, as Escape does: for the slot plugin, which
    /// takes a dialog down when what it asked for failed.
    pub(crate) fn back(&mut self) -> bool {
        self.pop()
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
    /// The Save page opened: list the slots.
    OpenSave,
    /// The Load page opened: list the slots.
    OpenLoad,
    /// Save to this row of the Save page; a new slot takes the name
    /// field's text (M4-11).
    SaveSlot(SaveChoice),
    /// Load the slot at this place of the list.
    LoadSlot(u16),
    /// Rename the slot at this place to the name field's text.
    RenameSlot(u16),
    /// Delete the slot at this place.
    DeleteSlot(u16),
    /// Restart at the title: the game starts again with no key waiting,
    /// so the whole intro plays (3.2.5; M4-12).
    QuitToTitle,
    /// Close the game (M4-07).
    QuitToDesktop,
    /// The title layer's Continue without a slot: close the layer and press
    /// the game's own Journey Onward (`J`) on the working save (3.2.4).
    JourneyOnward,
    /// Flip the check box of the setting with this catalogue key.
    Flip(&'static str),
    /// Step the setting with this catalogue key down (-1) or up (1).
    Step(&'static str, i8),
    /// Choose the preset at this place of the preset row (3.2.7): every
    /// row it governs takes its value.
    Preset(u8),
    /// Close the menu and type the game's own key for `GameKey` at the
    /// idle prompt (3.2.7 "Game").
    TypeGameKey(GameKey),
}

/// Applies `input` to `model` for the shell in `snap` and returns the
/// commands it gives (at most one). See the module notes for the keys.
pub fn reduce(model: &mut MenuModel, input: MenuInput, snap: &ShellSnapshot) -> Vec<MenuCommand> {
    let state = model.current();
    let command = match state.page {
        Page::Save => reduce_save(model, input, snap),
        Page::Load => reduce_load(model, input, snap),
        Page::Dialog(dialog) => reduce_dialog(model, dialog, input),
        Page::Top | Page::Title | Page::Settings(_) => return reduce_list(model, input, snap),
    };
    command.into_iter().collect()
}

/// The slot pages' common keys: F12 closes, Escape goes back, Up and Down
/// move the cursor over `len` rows. `None` for any other key.
fn common_key(model: &mut MenuModel, key: KeyCode, len: usize) -> Option<Option<MenuCommand>> {
    let cursor = model.current().cursor.min(len.saturating_sub(1));
    model.top_mut().cursor = cursor;
    match key {
        KeyCode::F12 => Some(Some(MenuCommand::Close)),
        KeyCode::Escape => Some((!model.pop()).then_some(MenuCommand::Close)),
        KeyCode::ArrowUp => {
            model.top_mut().cursor = cursor.saturating_sub(1);
            Some(None)
        }
        KeyCode::ArrowDown => {
            model.top_mut().cursor = (cursor + 1).min(len.saturating_sub(1));
            Some(None)
        }
        _ => None,
    }
}

fn is_enter(key: KeyCode) -> bool {
    matches!(key, KeyCode::Enter | KeyCode::NumpadEnter)
}

/// The Save page (3.2.2).
fn reduce_save(
    model: &mut MenuModel,
    input: MenuInput,
    snap: &ShellSnapshot,
) -> Option<MenuCommand> {
    let MenuInput::Key(key) = input else {
        return None;
    };
    let choices = save_choices(snap);
    if let Some(command) = common_key(model, key.key, choices.len()) {
        return command;
    }
    if !is_enter(key.key) {
        return None;
    }
    match *choices.get(model.current().cursor)? {
        SaveChoice::New => {
            model.field.clear();
            model.push(Page::Dialog(Dialog::Name(NameFor::New)));
            None
        }
        SaveChoice::Quick => Some(MenuCommand::SaveSlot(SaveChoice::Quick)),
        SaveChoice::Existing(at) => {
            model.push_at(Page::Dialog(Dialog::Overwrite(at)), SECOND_BUTTON);
            None
        }
    }
}

/// The Load page (3.2.3).
fn reduce_load(
    model: &mut MenuModel,
    input: MenuInput,
    snap: &ShellSnapshot,
) -> Option<MenuCommand> {
    let MenuInput::Key(key) = input else {
        return None;
    };
    if let Some(command) = common_key(model, key.key, snap.slot_list.len()) {
        return command;
    }
    let at = model.current().cursor;
    let slot = snap.slot_list.get(at)?;
    let place = at as u16;
    let damaged = slot.info.damaged.is_some();
    match key.key {
        k if is_enter(k) && !damaged => leave(model, Leave::Load(place), snap),
        KeyCode::F2 if !damaged && !slot.info.quick => {
            model.field = slot.info.name.chars().filter(|ch| name_char(*ch)).collect();
            model.field.truncate(t::SLOT_NAME_MAX);
            model.push(Page::Dialog(Dialog::Name(NameFor::Rename(place))));
            None
        }
        KeyCode::Delete => {
            model.push_at(Page::Dialog(Dialog::Delete(place)), SECOND_BUTTON);
            None
        }
        _ => None,
    }
}

/// The dialogs: the name field and the two questions. "Loading..." takes
/// no key: the restore runs on the next frame and the app ends.
fn reduce_dialog(model: &mut MenuModel, dialog: Dialog, input: MenuInput) -> Option<MenuCommand> {
    if let Dialog::Loading(_) = dialog {
        return None;
    }
    if let Dialog::Name(purpose) = dialog {
        return reduce_name(model, purpose, input);
    }
    let MenuInput::Key(key) = input else {
        return None;
    };
    match key.key {
        KeyCode::F12 => Some(MenuCommand::Close),
        KeyCode::Escape => {
            model.pop();
            None
        }
        KeyCode::ArrowLeft => {
            model.top_mut().cursor = 0;
            None
        }
        KeyCode::ArrowRight => {
            model.top_mut().cursor = SECOND_BUTTON;
            None
        }
        k if is_enter(k) => {
            let first = model.current().cursor == 0;
            model.pop();
            match dialog {
                Dialog::Overwrite(at) if first => {
                    Some(MenuCommand::SaveSlot(SaveChoice::Existing(at)))
                }
                Dialog::Delete(at) if first => Some(MenuCommand::DeleteSlot(at)),
                Dialog::Unsaved(what) if first => Some(go(model, what)),
                _ => None,
            }
        }
        _ => None,
    }
}

/// The name field (3.2.2): typed printable ASCII up to the cap, Backspace,
/// Enter and Escape. Every other key is ignored, so a letter's key press
/// (its text arrives as [`MenuInput::Text`]) does nothing of its own.
fn reduce_name(model: &mut MenuModel, purpose: NameFor, input: MenuInput) -> Option<MenuCommand> {
    let key = match input {
        MenuInput::Text(ch) => {
            if name_char(ch) && model.field.len() < t::SLOT_NAME_MAX {
                model.field.push(ch);
            }
            return None;
        }
        MenuInput::Key(key) => key.key,
    };
    match key {
        KeyCode::F12 => Some(MenuCommand::Close),
        KeyCode::Escape => {
            model.pop();
            None
        }
        KeyCode::Backspace => {
            model.field.pop();
            None
        }
        k if is_enter(k) => match purpose {
            NameFor::New => Some(MenuCommand::SaveSlot(SaveChoice::New)),
            NameFor::Rename(at) if !model.field.trim().is_empty() => {
                model.pop();
                Some(MenuCommand::RenameSlot(at))
            }
            NameFor::Rename(_) => None,
        },
        _ => None,
    }
}

/// The top menu and the Settings tabs.
fn reduce_list(model: &mut MenuModel, input: MenuInput, snap: &ShellSnapshot) -> Vec<MenuCommand> {
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
        KeyCode::Enter | KeyCode::NumpadEnter => selected.and_then(|row| choose(model, row, snap)),
        KeyCode::Space => selected.and_then(|row| match (row.kind, row.action) {
            (RowKind::Check { .. }, RowAction::Setting(key)) => Some(MenuCommand::Flip(key)),
            _ => None,
        }),
        KeyCode::ArrowLeft | KeyCode::ArrowRight => {
            let step = if key.key == KeyCode::ArrowLeft { -1 } else { 1 };
            // A greyed value cannot change, so on it the keys switch the
            // tab as on a row without a value.
            // The game's own Sound and Buffer show a value but change only
            // by their key (Enter), so on them too the keys switch the tab.
            let value = rows.get(cursor).filter(|row| {
                row.enabled
                    && matches!(row.action, RowAction::Setting(_) | RowAction::Preset)
                    && matches!(row.kind, RowKind::Value(_) | RowKind::Preset { .. })
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
                (None, _) => None,
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
fn choose(model: &mut MenuModel, row: &Row, snap: &ShellSnapshot) -> Option<MenuCommand> {
    match (row.action, row.kind) {
        (RowAction::Resume, _) => Some(MenuCommand::Close),
        (RowAction::Continue(Some(at)), _) => {
            model.push(Page::Dialog(Dialog::Loading(at)));
            Some(MenuCommand::LoadSlot(at))
        }
        (RowAction::Continue(None), _) => Some(MenuCommand::JourneyOnward),
        (RowAction::Save, _) => {
            model.push(Page::Save);
            Some(MenuCommand::OpenSave)
        }
        (RowAction::Load, _) => {
            model.push(Page::Load);
            Some(MenuCommand::OpenLoad)
        }
        (RowAction::Door(id), _) => Some(MenuCommand::OpenDoor(id)),
        (RowAction::Settings, _) => {
            model.push(Page::Settings(SettingsTab::Audio));
            None
        }
        (RowAction::QuitToTitle, _) => leave(model, Leave::Title, snap),
        (RowAction::QuitToDesktop, _) => leave(model, Leave::Desktop, snap),
        (RowAction::GameKey(key), _) => Some(MenuCommand::TypeGameKey(key)),
        (RowAction::Setting(key), RowKind::Check { .. }) => Some(MenuCommand::Flip(key)),
        (RowAction::Setting(_) | RowAction::Preset | RowAction::None, _) => None,
    }
}

/// Load, Return to the title or Leave the game, chosen: with unsaved
/// progress the question first, its second button (Cancel, No) chosen, so
/// Enter at once loses nothing (3.2.6); otherwise at once.
fn leave(model: &mut MenuModel, what: Leave, snap: &ShellSnapshot) -> Option<MenuCommand> {
    if snap.unsaved.is_some() {
        model.push_at(Page::Dialog(Dialog::Unsaved(what)), SECOND_BUTTON);
        return None;
    }
    Some(go(model, what))
}

/// Carries out `what`: a load shows "Loading..." while the host restores
/// the slot.
fn go(model: &mut MenuModel, what: Leave) -> MenuCommand {
    match what {
        Leave::Load(at) => {
            model.push(Page::Dialog(Dialog::Loading(at)));
            MenuCommand::LoadSlot(at)
        }
        Leave::Title => MenuCommand::QuitToTitle,
        Leave::Desktop => MenuCommand::QuitToDesktop,
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

//! The Save and Load pages with a fake host (design 3.2.2, 3.2.3, 3.3.5;
//! plan M4-11): the reducer's keys and the name field, the store and the
//! restore with every error toast, the load's relaunch request with
//! nothing written after the restore, the turns since the last save at
//! its three moments, and the slots trajectory test (G3).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc;

use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use u5_runtime::enhancements::{SHIPPED_PHASES, Toggle};
use u5_runtime::test_fixtures::{
    debug_game_dir, open_grid, saved_game_seed_bytes, test_state, write_empty_ool_mirrors,
};
use u5_runtime::{
    DisplayDriverFamily, PlayState, PreFlourishOutcome, TextWindowSystem,
    load_play_options_from_save, run_intro_pre_flourish_phase,
};

use super::*;
use crate::enh::VisualLaunchConfig;
use crate::enh::equivalence::{FrameRecord, Owner, Scene, Shell, TimedKey, Timeline};
use crate::enh::input::{KeyChord, OverlayKey};
use crate::enh::overlay::{OverlayId, OverlayState};
use crate::enh::synthetic::SyntheticOwner;
use crate::menu::host::SlotId;
use crate::menu::inject::SlotSavePhase;
use crate::menu::pages::{
    Dialog, MenuInput, MenuModel, NameFor, Page, PageState, SECOND_BUTTON, ShellSnapshot, reduce,
    save_choices,
};
use crate::menu::thumb::{THUMB_HEIGHT, THUMB_WIDTH};
use crate::menu::{MenuShell, world_snapshot};

/// The six working-set files a slot copies (u5-host-desktop's
/// `WORKING_SET_FILES`; the engine does not depend on the host crate).
const WORKING_SET: [&str; 6] = [
    "SAVED.GAM",
    "SAVED.OOL",
    "BRIT.OOL",
    "UNDER.OOL",
    "SAVED.WPS",
    ".u5-engine-town-npc-mutations",
];

type Files = BTreeMap<String, Vec<u8>>;

/// The working set in `dir`, by name.
fn working_files(dir: &Path) -> Files {
    WORKING_SET
        .iter()
        .filter_map(|name| {
            fs::read(dir.join(name))
                .ok()
                .map(|bytes| (name.to_string(), bytes))
        })
        .collect()
}

/// Makes the working set in `dir` exactly `files`.
fn write_working(dir: &Path, files: &Files) {
    for name in WORKING_SET {
        let _ = fs::remove_file(dir.join(name));
    }
    for (name, bytes) in files {
        fs::write(dir.join(name), bytes).unwrap();
    }
}

// --- the fake host ----------------------------------------------------------

/// What the fake host is told to get wrong.
#[derive(Default)]
struct Faults {
    list: Option<SlotFailure>,
    store: Option<SlotFailure>,
    /// The store's worker goes away without an answer.
    store_lost: bool,
    restore: Option<SlotFailure>,
    /// The restore "succeeds" but leaves a save with no game in it.
    restore_empty: bool,
    rename: Option<SlotFailure>,
    delete: Option<SlotFailure>,
}

/// A slot store in memory over the working folder `dir`: it copies the
/// six files byte for byte, as the desktop store does.
struct FakeHost {
    dir: PathBuf,
    slots: Mutex<Vec<(SlotInfo, Files)>>,
    faults: Mutex<Faults>,
    stores: Mutex<Vec<StoreSlot>>,
    backup: Mutex<Option<Files>>,
    roll_backs: Mutex<u32>,
    clock: Mutex<u64>,
}

impl FakeHost {
    fn new(dir: &Path) -> Arc<Self> {
        Arc::new(Self {
            dir: dir.to_path_buf(),
            slots: Mutex::new(Vec::new()),
            faults: Mutex::new(Faults::default()),
            stores: Mutex::new(Vec::new()),
            backup: Mutex::new(None),
            roll_backs: Mutex::new(0),
            clock: Mutex::new(1_000),
        })
    }

    fn faults(&self) -> std::sync::MutexGuard<'_, Faults> {
        self.faults.lock().unwrap()
    }

    fn roll_backs(&self) -> u32 {
        *self.roll_backs.lock().unwrap()
    }

    fn stores(&self) -> Vec<StoreSlot> {
        self.stores.lock().unwrap().clone()
    }

    fn slot_files(&self, name: &str) -> Files {
        let slots = self.slots.lock().unwrap();
        slots
            .iter()
            .find(|(info, _)| info.name == name)
            .map(|(_, files)| files.clone())
            .expect("the slot exists")
    }

    fn names(&self) -> Vec<String> {
        self.list_slots()
            .unwrap()
            .into_iter()
            .map(|slot| slot.name)
            .collect()
    }

    /// A slot made without the menu: an ordinary one or a damaged one.
    fn add(&self, name: &str, damaged: bool) {
        let mut slots = self.slots.lock().unwrap();
        let id = format!("s{:04}", slots.len() + 1);
        let mut clock = self.clock.lock().unwrap();
        *clock += 1;
        slots.push((
            SlotInfo {
                id: SlotId(id),
                name: name.to_string(),
                damaged: damaged.then(|| "SAVED.GAM is missing".to_string()),
                avatar: "Ann".to_string(),
                place: "Britannia".to_string(),
                clock: [139, 4, 7, 9, 15],
                turn: 7,
                saved_at_unix: *clock,
                ..SlotInfo::default()
            },
            working_files(&self.dir),
        ));
    }
}

impl MenuHost for FakeHost {
    fn list_slots(&self) -> Result<Vec<SlotInfo>, SlotFailure> {
        if let Some(failure) = self.faults().list.clone() {
            return Err(failure);
        }
        let mut slots: Vec<SlotInfo> = self
            .slots
            .lock()
            .unwrap()
            .iter()
            .map(|(info, _)| info.clone())
            .collect();
        slots.sort_by(|a, b| {
            b.quick
                .cmp(&a.quick)
                .then(b.saved_at_unix.cmp(&a.saved_at_unix))
        });
        Ok(slots)
    }

    fn store_slot(&self, request: StoreSlot) -> Receiver<Result<SlotInfo, SlotFailure>> {
        let (send, reply) = mpsc::channel();
        self.stores.lock().unwrap().push(request.clone());
        if self.faults().store_lost {
            drop(send);
            return reply;
        }
        if let Some(failure) = self.faults().store.clone() {
            send.send(Err(failure)).unwrap();
            return reply;
        }
        let files = working_files(&self.dir);
        let mut slots = self.slots.lock().unwrap();
        let mut clock = self.clock.lock().unwrap();
        *clock += 1;
        let at = match &request.target {
            SlotTarget::Existing(id) => slots.iter().position(|(info, _)| info.id == *id),
            SlotTarget::Quick => slots.iter().position(|(info, _)| info.quick),
            SlotTarget::New(_) => None,
        };
        let (id, name, quick) = match (&request.target, at) {
            (_, Some(at)) => {
                let info = &slots[at].0;
                (info.id.clone(), info.name.clone(), info.quick)
            }
            (SlotTarget::Quick, None) => (SlotId("quick".into()), "quick".into(), true),
            (SlotTarget::New(name), None) => {
                let number = slots.len() + 1;
                let name = if name.trim().is_empty() {
                    format!("Slot {number}")
                } else {
                    name.clone()
                };
                (SlotId(format!("s{number:04}")), name, false)
            }
            (SlotTarget::Existing(_), None) => unreachable!("the test stores to a listed slot"),
        };
        let info = SlotInfo {
            id,
            name,
            quick,
            avatar: request.avatar.clone(),
            place: request.place.clone(),
            clock: request.clock,
            turn: request.turn,
            saved_at_unix: *clock,
            thumbnail_png: request.thumbnail_png.clone(),
            ..SlotInfo::default()
        };
        match at {
            Some(at) => slots[at] = (info.clone(), files),
            None => slots.push((info.clone(), files)),
        }
        send.send(Ok(info)).unwrap();
        reply
    }

    fn restore_slot(&self, id: &SlotId) -> Result<(), SlotFailure> {
        if let Some(failure) = self.faults().restore.clone() {
            return Err(failure);
        }
        let files = {
            let slots = self.slots.lock().unwrap();
            let (_, files) = slots
                .iter()
                .find(|(info, _)| info.id == *id)
                .ok_or_else(|| SlotFailure::Reason("no such slot".into()))?;
            files.clone()
        };
        *self.backup.lock().unwrap() = Some(working_files(&self.dir));
        write_working(&self.dir, &files);
        if self.faults().restore_empty {
            fs::write(
                self.dir.join(SAVED_GAM_FILENAME),
                vec![0u8; u5_runtime::SAVED_GAM_LEN],
            )
            .unwrap();
        }
        Ok(())
    }

    fn roll_back_restore(&self) -> Result<(), SlotFailure> {
        *self.roll_backs.lock().unwrap() += 1;
        let backup = self.backup.lock().unwrap().clone();
        let backup = backup.ok_or_else(|| SlotFailure::Reason("no load to undo".into()))?;
        write_working(&self.dir, &backup);
        Ok(())
    }

    fn rename_slot(&self, id: &SlotId, name: &str) -> Result<(), SlotFailure> {
        if let Some(failure) = self.faults().rename.clone() {
            return Err(failure);
        }
        let mut slots = self.slots.lock().unwrap();
        let slot = slots.iter_mut().find(|(info, _)| info.id == *id).unwrap();
        slot.0.name = name.to_string();
        Ok(())
    }

    fn delete_slot(&self, id: &SlotId) -> Result<(), SlotFailure> {
        if let Some(failure) = self.faults().delete.clone() {
            return Err(failure);
        }
        self.slots
            .lock()
            .unwrap()
            .retain(|(info, _)| info.id != *id);
        Ok(())
    }
}

// --- the game ---------------------------------------------------------------

/// A scratch game folder that loads: the fixture's data files, a seed save
/// in Castle British and the two mirrors. Removed on drop.
struct GameDir(PathBuf);

impl GameDir {
    fn new() -> Self {
        let dir = debug_game_dir();
        fs::write(
            dir.join(SAVED_GAM_FILENAME),
            saved_game_seed_bytes(0x11, 0, 5, 5),
        )
        .unwrap();
        write_empty_ool_mirrors(&dir);
        Self(dir)
    }

    /// A fresh folder with `files` as its working set.
    fn with_working(files: &Files) -> Self {
        let dir = Self::new();
        write_working(&dir.0, files);
        dir
    }

    /// The game its working save loads into (once a save wrote an Avatar
    /// into it: the seed holds none).
    fn state(&self) -> PlayState {
        let options = load_play_options_from_save(&self.0).expect("the seed save loads");
        PlayState::load_scene(&self.0, options).expect("the seed scene loads")
    }
}

impl Drop for GameDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// The game the runs start in: the synthetic town at (5, 5).
fn town() -> PlayState {
    test_state(open_grid(), 5, 5)
}

fn scene(dir: &Path) -> Scene {
    Scene {
        game_dir: dir.to_path_buf(),
        ..Scene::synthetic()
    }
}

/// The menu with named slots and `host`, with the save folder for the save
/// watcher.
fn launch(host: &Arc<FakeHost>, dir: &Path) -> VisualLaunchConfig {
    let mut launch = VisualLaunchConfig::classic();
    launch.enhancements.toggles.set(Toggle::MenuOverlay, true);
    launch
        .enhancements
        .toggles
        .set(Toggle::MenuNamedSlots, true);
    launch.menu_host = Some(host.clone() as Arc<dyn MenuHost>);
    launch.save_dir = Some(dir.to_path_buf());
    launch
}

/// The headless app, as the default launch's shell (its top menu lists
/// Save and Load), with the exit slot a `_with` entry point would read.
struct Game {
    shell: Shell,
    exit: ShellExitSlot,
}

impl Game {
    fn new(state: PlayState, dir: &Path, launch: &VisualLaunchConfig) -> Self {
        let exit = ShellExitSlot::default();
        let slot = exit.clone();
        let slots = launch.menu_host.is_some();
        let shell = Shell::new(state, &scene(dir), launch, move |app| {
            app.insert_resource(slot);
            app.insert_resource(MenuShell {
                intro_shell: true,
                slots,
                shipped: SHIPPED_PHASES,
            });
        });
        Self { shell, exit }
    }

    fn step(&mut self, events: &[TimedKey]) -> FrameRecord {
        self.shell.step(events)
    }

    fn idle(&mut self, frames: usize) {
        for _ in 0..frames {
            self.step(&[]);
        }
    }

    /// A key pressed one frame and released the next.
    fn tap(&mut self, key: KeyCode) {
        let event = |state| TimedKey {
            key,
            state,
            repeat: false,
            owner: Owner::Game,
        };
        self.step(&[event(ButtonState::Pressed)]);
        self.step(&[event(ButtonState::Released)]);
    }

    /// `text` typed as a layout gives it: `key` with the character as its
    /// text.
    fn type_char(&mut self, key: KeyCode, text: &str) {
        for state in [ButtonState::Pressed, ButtonState::Released] {
            self.shell.app.world_mut().send_event(KeyboardInput {
                key_code: key,
                logical_key: Key::Character(text.into()),
                state,
                text: (state == ButtonState::Pressed).then(|| text.into()),
                repeat: false,
                window: Entity::PLACEHOLDER,
            });
            self.shell.app.update();
        }
    }

    /// `name` typed on a US layout.
    fn type_name(&mut self, name: &str) {
        for ch in name.chars() {
            let key = match ch {
                ' ' => KeyCode::Space,
                _ => KeyChord::parse(&ch.to_ascii_uppercase().to_string())
                    .map_or(KeyCode::Minus, |chord| chord.key),
            };
            self.type_char(key, &ch.to_string());
        }
    }

    fn world(&self) -> &World {
        self.shell.app.world()
    }

    fn model(&self) -> Option<MenuModel> {
        self.world().resource::<MenuState>().model().cloned()
    }

    fn page(&self) -> Option<Page> {
        self.model().map(|model| model.current().page)
    }

    fn menu_open(&self) -> bool {
        self.world().resource::<OverlayState>().open() == Some(OverlayId::Menu)
    }

    fn toast(&self) -> Option<String> {
        self.world()
            .resource::<Toasts>()
            .current()
            .map(str::to_string)
    }

    fn turn(&self) -> u64 {
        self.shell.visual().state.turn
    }

    fn tracker(&self) -> SaveTracker {
        *self.world().resource::<SaveTracker>()
    }

    fn app_exit(&self) -> bool {
        self.world()
            .get_resource::<Events<AppExit>>()
            .is_some_and(|events| !events.is_empty())
    }

    /// Opens the menu at the idle prompt and chooses top-menu row `row`
    /// (1 Save, 2 Load).
    fn open_row(&mut self, row: usize) {
        self.idle(4);
        self.tap(KeyCode::F12);
        assert!(self.menu_open(), "the menu opens at the idle prompt");
        for _ in 0..row {
            self.tap(KeyCode::ArrowDown);
        }
        self.tap(KeyCode::Enter);
    }

    /// Runs until the save and its copy are over.
    fn finish_save(&mut self) {
        for _ in 0..400 {
            self.step(&[]);
            let job = self.world().resource::<SlotJob>();
            let handshake = self.world().resource::<SlotSaveHandshake>();
            if !job.saving() && !handshake.busy() {
                return;
            }
        }
        panic!("the save did not finish");
    }

    /// Saves to a new slot named `name` through the menu.
    fn save_new(&mut self, name: &str) {
        self.open_row(1);
        assert_eq!(self.page(), Some(Page::Save));
        self.tap(KeyCode::Enter);
        assert_eq!(self.page(), Some(Page::Dialog(Dialog::Name(NameFor::New))));
        self.type_name(name);
        assert_eq!(self.model().unwrap().field(), name);
        self.tap(KeyCode::Enter);
        self.finish_save();
    }

    /// The player's own `Q` and `Y`, each when the game asks.
    fn typed_save(&mut self) {
        self.idle(4);
        self.tap(KeyCode::KeyQ);
        for _ in 0..200 {
            if save_prompt(self) {
                break;
            }
            self.step(&[]);
        }
        assert!(save_prompt(self), "Q opens the save prompt");
        self.tap(KeyCode::KeyY);
        for _ in 0..200 {
            if self.shell.visual().state.at_idle_command_prompt() && !save_prompt(self) {
                break;
            }
            self.step(&[]);
        }
        self.idle(4);
    }

    /// One step north: a turn of the game.
    fn walk(&mut self) {
        self.idle(4);
        self.tap(KeyCode::ArrowUp);
        self.idle(20);
    }

    /// Opens Load and chooses the slot at `at` of the list.
    fn load(&mut self, at: usize) {
        self.open_row(2);
        assert_eq!(self.page(), Some(Page::Load));
        for _ in 0..at {
            self.tap(KeyCode::ArrowDown);
        }
        self.tap(KeyCode::Enter);
    }
}

fn save_prompt(game: &Game) -> bool {
    game.shell
        .visual()
        .state
        .active_yes_no_prompt
        .map(|session| session.kind)
        == Some(u5_runtime::z_stats::YesNoPromptKind::SaveGame)
}

// --- the reducer --------------------------------------------------------------

fn key(code: KeyCode) -> MenuInput {
    MenuInput::Key(OverlayKey::plain(code))
}

fn snap_with(slots: Vec<SlotInfo>) -> ShellSnapshot {
    ShellSnapshot {
        intro_shell: true,
        slots: true,
        save_available: true,
        quicksave: true,
        slot_list: Arc::new(slots.into_iter().map(slot_entry).collect()),
        ..ShellSnapshot::default()
    }
}

fn info(id: &str, name: &str) -> SlotInfo {
    SlotInfo {
        id: SlotId(id.into()),
        name: name.into(),
        quick: id == "quick",
        ..SlotInfo::default()
    }
}

/// A model on the Save or Load page, reached as a player reaches it, with
/// the cursor moved down `cursor` rows.
fn model_at(page: Page, cursor: usize) -> MenuModel {
    let mut model = MenuModel::new();
    let snap = snap_with(Vec::new());
    let row = match page {
        Page::Save => 1,
        Page::Load => 2,
        _ => unreachable!(),
    };
    for _ in 0..row {
        reduce(&mut model, key(KeyCode::ArrowDown), &snap);
    }
    reduce(&mut model, key(KeyCode::Enter), &snap);
    assert_eq!(model.current().page, page);
    let four = snap_with(vec![
        info("quick", "quick"),
        info("s0001", "a"),
        info("s0002", "b"),
        info("s0003", "c"),
    ]);
    for _ in 0..cursor {
        reduce(&mut model, key(KeyCode::ArrowDown), &four);
    }
    model
}

#[test]
fn the_top_menu_opens_save_and_load_as_pages() {
    let snap = snap_with(Vec::new());
    let mut model = MenuModel::new();
    reduce(&mut model, key(KeyCode::ArrowDown), &snap);
    assert_eq!(
        reduce(&mut model, key(KeyCode::Enter), &snap),
        [MenuCommand::OpenSave]
    );
    assert_eq!(model.current().page, Page::Save);
    assert_eq!(reduce(&mut model, key(KeyCode::Escape), &snap), []);
    assert_eq!(model.current().page, Page::Top);
    reduce(&mut model, key(KeyCode::ArrowDown), &snap);
    assert_eq!(
        reduce(&mut model, key(KeyCode::Enter), &snap),
        [MenuCommand::OpenLoad]
    );
    assert_eq!(model.current().page, Page::Load);
}

#[test]
fn the_save_page_offers_new_quick_and_every_whole_slot() {
    let mut damaged = info("s0003", "broken");
    damaged.damaged = Some("missing".into());
    let snap = snap_with(vec![info("quick", "quick"), info("s0001", "a"), damaged]);
    assert_eq!(
        save_choices(&snap),
        [SaveChoice::New, SaveChoice::Quick, SaveChoice::Existing(1)]
    );
    let off = ShellSnapshot {
        quicksave: false,
        ..snap.clone()
    };
    assert_eq!(
        save_choices(&off),
        [SaveChoice::New, SaveChoice::Existing(1)],
        "no quick row with menu.quicksave off"
    );
    // New opens the name field.
    let mut model = model_at(Page::Save, 0);
    assert_eq!(reduce(&mut model, key(KeyCode::Enter), &snap), []);
    assert_eq!(
        model.current().page,
        Page::Dialog(Dialog::Name(NameFor::New))
    );
    // The quick slot saves at once.
    let mut model = model_at(Page::Save, 0);
    reduce(&mut model, key(KeyCode::ArrowDown), &snap);
    assert_eq!(
        reduce(&mut model, key(KeyCode::Enter), &snap),
        [MenuCommand::SaveSlot(SaveChoice::Quick)]
    );
    // A slot asks first, with No chosen.
    let mut model = model_at(Page::Save, 0);
    reduce(&mut model, key(KeyCode::ArrowDown), &snap);
    reduce(&mut model, key(KeyCode::ArrowDown), &snap);
    assert_eq!(reduce(&mut model, key(KeyCode::Enter), &snap), []);
    assert_eq!(
        model.current(),
        PageState {
            page: Page::Dialog(Dialog::Overwrite(1)),
            cursor: SECOND_BUTTON
        }
    );
    // Enter on No goes back and writes nothing; Yes writes over it.
    assert_eq!(reduce(&mut model, key(KeyCode::Enter), &snap), []);
    assert_eq!(model.current().page, Page::Save);
    reduce(&mut model, key(KeyCode::Enter), &snap);
    reduce(&mut model, key(KeyCode::ArrowLeft), &snap);
    assert_eq!(
        reduce(&mut model, key(KeyCode::Enter), &snap),
        [MenuCommand::SaveSlot(SaveChoice::Existing(1))]
    );
    // The cursor stops at the ends; F12 closes from anywhere.
    let mut model = model_at(Page::Save, 0);
    for _ in 0..9 {
        reduce(&mut model, key(KeyCode::ArrowDown), &snap);
    }
    assert_eq!(model.current().cursor, 2);
    assert_eq!(
        reduce(&mut model, key(KeyCode::F12), &snap),
        [MenuCommand::Close]
    );
    reduce(&mut model, key(KeyCode::Enter), &snap);
    assert_eq!(
        reduce(&mut model, key(KeyCode::F12), &snap),
        [MenuCommand::Close]
    );
}

/// The name field (3.2.2): printable ASCII only, at most 24 characters,
/// Backspace, a non-US layout's character refused, keys that are not text
/// ignored.
#[test]
fn the_name_field_takes_printable_ascii_up_to_the_cap() {
    let snap = snap_with(Vec::new());
    let mut model = model_at(Page::Save, 0);
    reduce(&mut model, key(KeyCode::Enter), &snap);
    for ch in "Ok, Day 1!".chars() {
        assert_eq!(reduce(&mut model, MenuInput::Text(ch), &snap), []);
    }
    assert_eq!(model.field(), "Ok, Day 1!");
    for ch in ['é', 'ß', 'Ж', '\t', '\u{7f}', '€'] {
        reduce(&mut model, MenuInput::Text(ch), &snap);
    }
    assert_eq!(model.field(), "Ok, Day 1!", "refused");
    // A letter's key press is not text, and neither is Space's key.
    for code in [
        KeyCode::KeyA,
        KeyCode::Space,
        KeyCode::ArrowLeft,
        KeyCode::Delete,
    ] {
        assert_eq!(reduce(&mut model, key(code), &snap), []);
    }
    assert_eq!(model.field(), "Ok, Day 1!");
    reduce(&mut model, key(KeyCode::Backspace), &snap);
    reduce(&mut model, key(KeyCode::Backspace), &snap);
    assert_eq!(model.field(), "Ok, Day ");
    for _ in 0..40 {
        reduce(&mut model, MenuInput::Text('x'), &snap);
    }
    assert_eq!(model.field().len(), t::SLOT_NAME_MAX, "the cap");
    for _ in 0..30 {
        reduce(&mut model, key(KeyCode::Backspace), &snap);
    }
    assert_eq!(model.field(), "");
    // Enter saves, even unnamed ("Slot N"); Escape goes back.
    assert_eq!(
        reduce(&mut model, key(KeyCode::Enter), &snap),
        [MenuCommand::SaveSlot(SaveChoice::New)]
    );
    assert_eq!(reduce(&mut model, key(KeyCode::Escape), &snap), []);
    assert_eq!(model.current().page, Page::Save);
    // A new name field starts empty.
    reduce(&mut model, MenuInput::Text('q'), &snap);
    reduce(&mut model, key(KeyCode::Enter), &snap);
    assert_eq!(model.field(), "");
}

#[test]
fn the_load_page_loads_renames_and_deletes() {
    let mut damaged = info("s0002", "broken");
    damaged.damaged = Some("missing".into());
    let snap = snap_with(vec![
        info("quick", "quick"),
        info("s0001", "Despise"),
        damaged,
    ]);
    let mut model = model_at(Page::Load, 0);
    // F2 on the quick slot: it keeps its name.
    assert_eq!(reduce(&mut model, key(KeyCode::F2), &snap), []);
    assert_eq!(model.current().page, Page::Load);
    // Enter on a slot shows "Loading..." and loads it.
    assert_eq!(
        reduce(&mut model, key(KeyCode::Enter), &snap),
        [MenuCommand::LoadSlot(0)]
    );
    assert_eq!(model.current().page, Page::Dialog(Dialog::Loading(0)));
    // While it loads no key does anything, F12 included.
    for code in [KeyCode::F12, KeyCode::Escape, KeyCode::Enter] {
        assert_eq!(reduce(&mut model, key(code), &snap), []);
    }
    assert_eq!(model.current().page, Page::Dialog(Dialog::Loading(0)));
    // F2 renames, starting from the old name; a blank name is refused.
    let mut model = model_at(Page::Load, 1);
    assert_eq!(reduce(&mut model, key(KeyCode::F2), &snap), []);
    assert_eq!(
        model.current().page,
        Page::Dialog(Dialog::Name(NameFor::Rename(1)))
    );
    assert_eq!(model.field(), "Despise");
    for _ in 0..7 {
        reduce(&mut model, key(KeyCode::Backspace), &snap);
    }
    reduce(&mut model, MenuInput::Text(' '), &snap);
    assert_eq!(reduce(&mut model, key(KeyCode::Enter), &snap), []);
    reduce(&mut model, MenuInput::Text('W'), &snap);
    assert_eq!(
        reduce(&mut model, key(KeyCode::Enter), &snap),
        [MenuCommand::RenameSlot(1)]
    );
    assert_eq!(model.current().page, Page::Load);
    // A damaged slot: no load, no rename; Delete asks, Cancel chosen.
    let mut model = model_at(Page::Load, 2);
    assert_eq!(reduce(&mut model, key(KeyCode::Enter), &snap), []);
    assert_eq!(reduce(&mut model, key(KeyCode::F2), &snap), []);
    assert_eq!(model.current().page, Page::Load);
    reduce(&mut model, key(KeyCode::Delete), &snap);
    assert_eq!(
        model.current(),
        PageState {
            page: Page::Dialog(Dialog::Delete(2)),
            cursor: SECOND_BUTTON
        }
    );
    assert_eq!(reduce(&mut model, key(KeyCode::Enter), &snap), []);
    assert_eq!(model.current().page, Page::Load, "Cancel");
    reduce(&mut model, key(KeyCode::Delete), &snap);
    reduce(&mut model, key(KeyCode::ArrowLeft), &snap);
    assert_eq!(
        reduce(&mut model, key(KeyCode::Enter), &snap),
        [MenuCommand::DeleteSlot(2)]
    );
    assert_eq!(model.current().page, Page::Load);
    // An empty list takes the keys and does nothing.
    let empty = snap_with(Vec::new());
    let mut model = model_at(Page::Load, 0);
    for code in [
        KeyCode::Enter,
        KeyCode::F2,
        KeyCode::Delete,
        KeyCode::ArrowDown,
    ] {
        assert_eq!(reduce(&mut model, key(code), &empty), []);
    }
    assert_eq!(model.current().page, Page::Load);
}

/// The top menu's last-save line (3.3.5 "Unsaved progress") fits a
/// narrow page's row however long the session.
#[test]
fn the_last_save_line_always_fits() {
    use crate::menu::render::last_save_line;
    assert_eq!(last_save_line(None), None);
    assert_eq!(last_save_line(Some(0)).as_deref(), Some(t::LAST_SAVE_NOW));
    assert_eq!(last_save_line(Some(1)).as_deref(), Some("1 turn unsaved."));
    assert_eq!(
        last_save_line(Some(37)).as_deref(),
        Some("37 turns unsaved.")
    );
    for turns in [0, 1, 9, 99_999, 100_000, u64::MAX] {
        let line = last_save_line(Some(turns)).unwrap();
        assert!(line.len() <= t::NARROW_TEXT, "{line}");
    }
}

// --- the slot plugin in the shell ---------------------------------------------

/// A save to a new slot is a real Q save, then a copy through the host:
/// the slot's details come from the save (the place in published words),
/// the thumbnail from the game's frame, and the toast names the slot.
#[test]
fn a_save_to_a_new_slot_stores_through_the_host() {
    let dir = GameDir::new();
    let host = FakeHost::new(&dir.0);
    let mut game = Game::new(town(), &dir.0, &launch(&host, &dir.0));
    let before = working_files(&dir.0);
    game.save_new("Morning walk");
    let stores = host.stores();
    let [store] = stores.as_slice() else {
        panic!("one store: {stores:?}");
    };
    assert_eq!(store.target, SlotTarget::New("Morning walk".into()));
    assert_eq!(store.place, "Castle British", "D-M4.8, never a scene key");
    assert_eq!(store.turn, game.turn());
    assert!(!store.avatar.is_empty());
    assert!(store.thumbnail_png.is_some());
    assert_eq!(game.toast().as_deref(), Some("Saved to \"Morning walk\"."));
    assert!(!game.menu_open(), "the menu closed for the save");
    let owner = game.world().resource::<SyntheticInputOwner>();
    assert_eq!(owner.holder(), None, "the token is released");
    assert_eq!(
        game.world().resource::<SlotSaveHandshake>().phase(),
        SlotSavePhase::Idle
    );
    // The game's own save wrote the working set, and the slot holds it.
    let after = working_files(&dir.0);
    assert_ne!(before, after);
    assert_eq!(host.slot_files("Morning walk"), after);
    // The listing has it, its thumbnail decoded.
    let listed = &game.world().resource::<SlotListing>().slots;
    assert_eq!(listed.len(), 1);
    assert_eq!(
        listed[0].thumb.as_ref().map(Vec::len),
        Some((THUMB_WIDTH * THUMB_HEIGHT * 4) as usize)
    );
    assert_eq!(game.tracker().last_saved_turn, Some(store.turn));
}

#[test]
fn an_unnamed_slot_the_quick_slot_and_writing_over_one() {
    let dir = GameDir::new();
    let host = FakeHost::new(&dir.0);
    let mut game = Game::new(town(), &dir.0, &launch(&host, &dir.0));
    game.save_new("");
    assert_eq!(game.toast().as_deref(), Some("Saved to \"Slot 1\"."));
    // The quick slot never asks.
    game.open_row(1);
    game.tap(KeyCode::ArrowDown);
    game.tap(KeyCode::Enter);
    game.finish_save();
    assert_eq!(game.toast().as_deref(), Some("Saved to \"quick\"."));
    assert_eq!(host.names(), ["quick", "Slot 1"]);
    // Writing over a slot asks, and goes on with Yes.
    game.open_row(1);
    game.tap(KeyCode::ArrowDown);
    game.tap(KeyCode::ArrowDown);
    game.tap(KeyCode::Enter);
    assert_eq!(game.page(), Some(Page::Dialog(Dialog::Overwrite(1))));
    game.tap(KeyCode::ArrowLeft);
    game.tap(KeyCode::Enter);
    game.finish_save();
    assert_eq!(game.toast().as_deref(), Some("Saved to \"Slot 1\"."));
    let targets: Vec<SlotTarget> = host.stores().into_iter().map(|s| s.target).collect();
    assert_eq!(
        targets,
        [
            SlotTarget::New(String::new()),
            SlotTarget::Quick,
            SlotTarget::Existing(SlotId("s0001".into())),
        ]
    );
}

/// Every way the copy can fail says so, names the slot, and releases the
/// token; the game's own save stands (3.2.2 "Errors").
#[test]
fn every_store_failure_has_its_toast() {
    type Setup = fn(&mut Faults);
    let cases: [(&str, Setup, &str); 4] = [
        (
            "Despise",
            |f| f.store = Some(SlotFailure::Busy),
            t::SLOTS_BUSY,
        ),
        (
            "Despise",
            |f| f.store = Some(SlotFailure::Reason("disk full".into())),
            "Saved the game, but could not write \"Despise\": disk full.",
        ),
        (
            "",
            |f| f.store = Some(SlotFailure::Reason("disk full".into())),
            "Saved the game, but could not write the new slot: disk full.",
        ),
        (
            "Despise",
            |f| f.store_lost = true,
            "Saved the game, but could not write \"Despise\": the copy stopped.",
        ),
    ];
    for (name, setup, toast) in cases {
        let dir = GameDir::new();
        let host = FakeHost::new(&dir.0);
        setup(&mut host.faults());
        let mut game = Game::new(town(), &dir.0, &launch(&host, &dir.0));
        let before = working_files(&dir.0);
        game.save_new(name);
        assert_eq!(game.toast().as_deref(), Some(toast), "{name:?}");
        assert!(host.names().is_empty(), "no slot was made");
        assert_ne!(working_files(&dir.0), before, "the Q save stands");
        let owner = game.world().resource::<SyntheticInputOwner>();
        assert_eq!(owner.holder(), None);
        // The Q save still counts as a save.
        assert_eq!(game.tracker().last_saved_turn, Some(game.turn()));
    }
}

/// A save asked for when the menu may not type is refused with its toast,
/// and nothing is typed.
#[test]
fn a_refused_save_says_why() {
    let dir = GameDir::new();
    let host = FakeHost::new(&dir.0);
    let mut game = Game::new(town(), &dir.0, &launch(&host, &dir.0));
    game.open_row(1);
    game.tap(KeyCode::Enter);
    // Another feature holds the token: can_save fails.
    game.shell
        .app
        .world_mut()
        .resource_mut::<SyntheticInputOwner>()
        .take(SyntheticOwner::StartKeys)
        .unwrap();
    game.type_name("A");
    game.tap(KeyCode::Enter);
    game.idle(3);
    assert_eq!(game.toast().as_deref(), Some(t::SAVE_UNAVAILABLE));
    assert!(host.stores().is_empty());
    assert!(!game.world().resource::<SlotJob>().saving());
    assert!(game.menu_open(), "nothing was typed; the menu stays");
}

/// A load restores the slot, asks the host for a relaunch with the boot
/// `J` and ends the app, all from one system; nothing writes the working
/// set after the restore (3.3.5 "Restore" step 4).
#[test]
fn a_load_asks_for_a_relaunch_and_writes_nothing_after_the_restore() {
    let dir = GameDir::new();
    let host = FakeHost::new(&dir.0);
    let mut game = Game::new(town(), &dir.0, &launch(&host, &dir.0));
    game.save_new("Despise");
    let slot = host.slot_files("Despise");
    game.walk();
    game.typed_save();
    assert_ne!(working_files(&dir.0), slot, "later progress was saved");
    game.load(0);
    // "Loading..." shows for a frame, then the restore runs.
    assert_eq!(game.page(), Some(Page::Dialog(Dialog::Loading(0))));
    assert!(game.app_exit(), "the app ends the frame after it shows");
    assert_eq!(
        game.exit.take(),
        ShellExit {
            relaunch: Some(Relaunch {
                boot_key: Some(b'J')
            })
        }
    );
    assert_eq!(working_files(&dir.0), slot, "the working set is the slot");
    game.idle(40);
    assert_eq!(working_files(&dir.0), slot, "nothing wrote it afterwards");
    assert!(game.menu_open(), "no key reached the game");
    assert_eq!(host.roll_backs(), 0);
}

/// A restore that fails, or leaves a working set the game cannot load,
/// puts it back and says why; the session goes on, on the Load page.
#[test]
fn every_load_failure_rolls_back_with_its_toast() {
    type Setup = fn(&mut Faults);
    let cases: [(Setup, &str, u32); 3] = [
        (|f| f.restore = Some(SlotFailure::Busy), t::SLOTS_BUSY, 0),
        (
            |f| f.restore = Some(SlotFailure::Reason("SAVED.GAM is missing".into())),
            "Could not load \"Despise\": SAVED.GAM is missing. Thy game is unchanged.",
            0,
        ),
        (
            |f| f.restore_empty = true,
            "Could not load \"Despise\": the save holds no game. Thy game is unchanged.",
            1,
        ),
    ];
    for (setup, toast, roll_backs) in cases {
        let dir = GameDir::new();
        let host = FakeHost::new(&dir.0);
        let mut game = Game::new(town(), &dir.0, &launch(&host, &dir.0));
        game.save_new("Despise");
        game.walk();
        game.typed_save();
        let before = working_files(&dir.0);
        setup(&mut host.faults());
        game.load(0);
        game.idle(2);
        assert_eq!(game.toast().as_deref(), Some(toast));
        assert_eq!(working_files(&dir.0), before, "the game is unchanged");
        assert_eq!(host.roll_backs(), roll_backs);
        assert!(!game.app_exit());
        assert_eq!(game.exit.take(), ShellExit::default());
        assert_eq!(game.page(), Some(Page::Load), "back on the Load page");
    }
}

/// A shell that cannot restart the game (no exit slot) undoes the load.
#[test]
fn a_load_without_a_relaunch_is_undone() {
    let dir = GameDir::new();
    let host = FakeHost::new(&dir.0);
    let mut game = Game::new(town(), &dir.0, &launch(&host, &dir.0));
    game.shell
        .app
        .world_mut()
        .remove_resource::<ShellExitSlot>();
    game.save_new("Despise");
    game.walk();
    game.typed_save();
    let before = working_files(&dir.0);
    game.load(0);
    game.idle(2);
    assert_eq!(working_files(&dir.0), before);
    assert_eq!(host.roll_backs(), 1);
    assert!(!game.app_exit());
    assert!(
        game.toast()
            .is_some_and(|toast| toast.contains("cannot restart"))
    );
}

#[test]
fn rename_delete_and_listing_say_what_happened() {
    let dir = GameDir::new();
    let host = FakeHost::new(&dir.0);
    host.add("Old game", false);
    host.add("Broken", true);
    let mut game = Game::new(town(), &dir.0, &launch(&host, &dir.0));
    game.open_row(2);
    // Newest first.
    let names: Vec<String> = game
        .world()
        .resource::<SlotListing>()
        .slots
        .iter()
        .map(|slot| slot.info.name.clone())
        .collect();
    assert_eq!(names, ["Broken", "Old game"]);
    game.tap(KeyCode::ArrowDown);
    game.tap(KeyCode::F2);
    for _ in 0..4 {
        game.tap(KeyCode::Backspace);
    }
    game.type_name("walk");
    game.tap(KeyCode::Enter);
    assert_eq!(game.toast().as_deref(), Some("Renamed to \"Old walk\"."));
    assert_eq!(host.names(), ["Broken", "Old walk"]);
    // A failing rename and delete name the slot and the reason.
    host.faults().rename = Some(SlotFailure::Reason("read-only".into()));
    game.tap(KeyCode::F2);
    game.type_name("x");
    game.tap(KeyCode::Enter);
    assert_eq!(
        game.toast().as_deref(),
        Some("Could not rename \"Old walk\": read-only.")
    );
    host.faults().delete = Some(SlotFailure::Reason("read-only".into()));
    game.tap(KeyCode::Delete);
    game.tap(KeyCode::ArrowLeft);
    game.tap(KeyCode::Enter);
    assert_eq!(
        game.toast().as_deref(),
        Some("Could not delete \"Old walk\": read-only.")
    );
    host.faults().delete = None;
    // The damaged slot can be deleted.
    game.tap(KeyCode::ArrowUp);
    game.tap(KeyCode::Delete);
    game.tap(KeyCode::ArrowLeft);
    game.tap(KeyCode::Enter);
    assert_eq!(game.toast().as_deref(), Some("Deleted \"Broken\"."));
    assert_eq!(host.names(), ["Old walk"]);
    assert_eq!(game.model().unwrap().current().cursor, 0);
    // A list that cannot be read is empty, with its toast.
    host.faults().list = Some(SlotFailure::Reason("no folder".into()));
    game.tap(KeyCode::Escape);
    game.tap(KeyCode::Enter);
    assert_eq!(
        game.toast().as_deref(),
        Some("Could not read the saves: no folder.")
    );
    assert!(world_snapshot(game.world()).slot_list.is_empty());
    host.faults().list = Some(SlotFailure::Busy);
    game.tap(KeyCode::Escape);
    game.tap(KeyCode::Enter);
    assert_eq!(game.toast().as_deref(), Some(t::SLOTS_BUSY));
}

/// The router never hands the menu a character outside printable ASCII,
/// whatever the layout typed (D-M4.2); the field takes the rest.
#[test]
fn a_non_us_layout_character_never_reaches_the_name_field() {
    let dir = GameDir::new();
    let host = FakeHost::new(&dir.0);
    let mut game = Game::new(town(), &dir.0, &launch(&host, &dir.0));
    game.open_row(1);
    game.tap(KeyCode::Enter);
    game.type_char(KeyCode::Semicolon, "ö");
    game.type_char(KeyCode::KeyE, "é");
    game.type_char(KeyCode::KeyA, "a");
    game.type_char(KeyCode::Digit2, "\"");
    assert_eq!(game.model().unwrap().field(), "a\"");
}

/// `last_saved_turn` at its three moments (3.3.5 "Unsaved progress"): the
/// first frame of play, a typed save the game answered, and a menu save.
#[test]
fn unsaved_progress_is_counted_from_the_three_moments() {
    let dir = GameDir::new();
    let host = FakeHost::new(&dir.0);
    let mut game = Game::new(town(), &dir.0, &launch(&host, &dir.0));
    game.idle(1);
    let start = game.turn();
    assert_eq!(game.tracker().last_saved_turn, Some(start), "moment 2");
    game.walk();
    game.walk();
    assert!(game.turn() > start);
    assert_eq!(
        world_snapshot(game.world()).turns_since_save,
        Some(game.turn() - start)
    );
    game.typed_save();
    assert_eq!(
        game.tracker().last_saved_turn,
        Some(game.turn()),
        "moment 3"
    );
    assert_eq!(world_snapshot(game.world()).turns_since_save, Some(0));
    game.walk();
    let walked = game.tracker().last_saved_turn;
    assert_ne!(walked, Some(game.turn()));
    game.save_new("A");
    let saved = host.stores()[0].turn;
    assert_eq!(game.tracker().last_saved_turn, Some(saved), "moment 1");
    // Opening and closing the menu moves nothing.
    game.idle(4);
    game.tap(KeyCode::F12);
    game.tap(KeyCode::F12);
    assert_eq!(game.tracker().last_saved_turn, Some(saved));
}

/// The count comes with the menu, slots or not, and a typed save counts
/// there too; without a save folder it still counts from the start.
#[test]
fn the_count_needs_no_slots() {
    let dir = GameDir::new();
    let mut launch = VisualLaunchConfig::classic();
    launch.enhancements.toggles.set(Toggle::MenuOverlay, true);
    launch.save_dir = Some(dir.0.clone());
    let mut game = Game::new(town(), &dir.0, &launch);
    assert!(
        !game.world().contains_resource::<SlotJob>(),
        "no slot plugin"
    );
    game.walk();
    game.typed_save();
    assert_eq!(game.tracker().last_saved_turn, Some(game.turn()));
    launch.save_dir = None;
    let mut game = Game::new(town(), &dir.0, &launch);
    game.idle(1);
    assert_eq!(game.tracker().last_saved_turn, Some(game.turn()));
}

/// The pre-flourish poll's answer for this boot key and these keys typed
/// as the window opens, as the intro app's builder asks it.
fn boot_poll(boot_key: Option<u8>, typed: &[char]) -> PreFlourishOutcome {
    let dir = debug_game_dir();
    let mut windows = TextWindowSystem::new();
    let (_, outcome) = run_intro_pre_flourish_phase(
        &dir,
        DisplayDriverFamily::Ega,
        &mut windows,
        crate::intro_boot_queued_key(boot_key, typed),
    )
    .unwrap();
    let _ = fs::remove_dir_all(&dir);
    outcome
}

/// G3, the slots trajectory test (plan M4-11 exit): a slot save, then a
/// load by relaunch with the boot `J`, against the same files restored by
/// hand and `J` typed at boot.
///
/// - The restore leaves the working set byte-equal to a hand copy of the
///   slot's files, and the relaunch asks for the boot `J`.
/// - The boot `J` takes the same pre-flourish path as a `J` the player
///   types as the window opens.
/// - The game that then starts from each folder runs the same: the same
///   trajectory digest, picture and key queue on every frame of a walk.
#[test]
fn a_relaunch_load_equals_a_hand_restore_and_a_typed_j() {
    let dir = GameDir::new();
    let host = FakeHost::new(&dir.0);
    let mut game = Game::new(town(), &dir.0, &launch(&host, &dir.0));
    game.walk();
    game.save_new("Despise");
    let slot = host.slot_files("Despise");
    game.walk();
    game.walk();
    game.typed_save();
    assert_ne!(
        working_files(&dir.0),
        slot,
        "the later save is another game"
    );
    game.load(0);
    game.idle(2);
    let asked = game.exit.take().relaunch.expect("a relaunch");
    drop(game);

    // By hand: the slot's files copied into a fresh game folder.
    let by_hand = GameDir::with_working(&slot);
    assert_eq!(working_files(&dir.0), working_files(&by_hand.0));

    // The relaunch's boot key is a J typed at boot.
    assert_eq!(asked.boot_key, Some(b'J'));
    assert_eq!(
        boot_poll(asked.boot_key, &[]),
        PreFlourishOutcome::JourneyOnwardShortcut
    );
    assert_eq!(
        boot_poll(None, &['J']),
        PreFlourishOutcome::JourneyOnwardShortcut
    );

    // The game each folder starts, walked the same way.
    let mut timeline = Timeline::new(120);
    for (i, code) in [KeyCode::ArrowUp, KeyCode::ArrowLeft, KeyCode::ArrowUp]
        .into_iter()
        .enumerate()
    {
        timeline.game_tap(10 + i * 30, code);
    }
    let run = |state: PlayState, folder: &Path| {
        let mut shell = Shell::new(
            state,
            &scene(folder),
            &VisualLaunchConfig::classic(),
            |_| {},
        );
        (0..timeline.len())
            .map(|frame| shell.step(timeline.frame(frame)))
            .collect::<Vec<_>>()
    };
    // Each start draws its seed from the host clock (`prng.md` §3, "Boot
    // placement"), the restart and the hand restore alike; the two runs
    // are given the same clock, as the paired harness pins it.
    let seeded = |game: &GameDir| {
        let mut state = game.state();
        state.prng_state = 0x5a5a;
        state
    };
    let restored = run(seeded(&dir), &dir.0);
    let hand = run(seeded(&by_hand), &by_hand.0);
    assert_eq!(restored.len(), hand.len());
    for (frame, (a, b)) in restored.iter().zip(&hand).enumerate() {
        assert_eq!(a, b, "frame {frame}");
    }
}

// --- the render check ---------------------------------------------------------

/// The M4-11 render check: the Save and Load pages and their dialogs in the
/// real shell over real engine frames, at 4:3 and on the grow-left 16:9
/// canvas (the column shut and open), for the user to hold beside the M4-01
/// sheet. Review output only, never committed, and `#[ignore]`d:
///
/// ```text
/// U5_MOCKUP_PROFILE=<copy> U5_MENU_SHOTS=<folder> \
///   cargo test -p u5-bevy --lib menu::slots::tests::slot_pages_review_shots -- --ignored
/// ```
///
/// The scene is the c04f forest road west of Britain at 11:50. The menu
/// saves twice through the game's own `Q` and `Y` (the slots' thumbnails
/// are the real frames), and the fake host adds a damaged slot, an import
/// and one saved with cheats, so every kind of row shows. Each shot is
/// written at the picture's size and at 2x, with 4x close-ups of the
/// panel's corners, tabs, junctions and bar ends, and whole-window shots
/// of the toasts.
#[test]
#[ignore = "review output: needs U5_MOCKUP_PROFILE and U5_MENU_SHOTS"]
fn slot_pages_review_shots() {
    use bevy::window::WindowResolution;
    use u5_runtime::test_fixtures::{configured_test_dir, copy_asset_writable, unique_scratch_dir};
    use u5_runtime::{GameClock, PlayTarget};

    use crate::menu::render::MenuView;
    use crate::menu::review_tests::{
        CASES, composed, crop, launch as review_launch, save_png, scaled, window_shot,
    };

    let (Some(profile), Some(out)) = (
        configured_test_dir("U5_MOCKUP_PROFILE"),
        configured_test_dir("U5_MENU_SHOTS"),
    ) else {
        eprintln!("skipped: set U5_MOCKUP_PROFILE and U5_MENU_SHOTS");
        return;
    };
    fs::create_dir_all(&out).unwrap();
    for case in [&CASES[0], &CASES[1], &CASES[2], &CASES[3]] {
        let game_dir = unique_scratch_dir("menu-slot-shots");
        for entry in fs::read_dir(&profile).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_file() {
                copy_asset_writable(&entry.path(), &game_dir.join(entry.file_name())).unwrap();
            }
        }
        let scene = Scene::from_game_dir(&game_dir).unwrap();
        let mut options = load_play_options_from_save(&game_dir).unwrap();
        options.target = PlayTarget::from_key("BRITANNIA").unwrap();
        options.saved_active_objects = None;
        options.start = Some((74, 111));
        options.clock = GameClock::new(11, 50).unwrap();
        let state = PlayState::load_scene(&game_dir, options).unwrap();
        let host = FakeHost::new(&game_dir);
        let mut launch = review_launch(case.grow);
        launch
            .enhancements
            .toggles
            .set(Toggle::MenuNamedSlots, true);
        launch.menu_host = Some(host.clone() as Arc<dyn MenuHost>);
        launch.save_dir = Some(game_dir.clone());
        let exit = ShellExitSlot::default();
        let slot = exit.clone();
        let shell = Shell::new(state, &scene, &launch, move |app| {
            app.insert_resource(slot);
            app.insert_resource(MenuShell {
                intro_shell: true,
                slots: true,
                shipped: SHIPPED_PHASES,
            });
        });
        let mut game = Game { shell, exit };
        game.shell.app.world_mut().spawn(Window {
            resolution: WindowResolution::new(case.window.0 as f32, case.window.1 as f32),
            ..default()
        });
        for key in [
            KeyCode::ArrowUp,
            KeyCode::ArrowUp,
            KeyCode::ArrowRight,
            KeyCode::ArrowRight,
            KeyCode::ArrowUp,
        ] {
            game.idle(10);
            game.tap(key);
        }
        game.idle(40);
        if case.column {
            let ctrl = |state| TimedKey {
                key: KeyCode::ControlLeft,
                state,
                repeat: false,
                owner: Owner::Feature,
            };
            let p = |state| TimedKey {
                key: KeyCode::KeyP,
                state,
                repeat: false,
                owner: Owner::Feature,
            };
            game.step(&[ctrl(ButtonState::Pressed), p(ButtonState::Pressed)]);
            game.step(&[p(ButtonState::Released), ctrl(ButtonState::Released)]);
            game.idle(40);
        }
        let label = case.label;
        let shot = |name: &str, (w, h, rgba): &(u32, u32, Vec<u8>)| {
            save_png(&out.join(format!("{label}-{name}.png")), *w, *h, rgba);
            save_png(
                &out.join(format!("{label}-{name}-2x.png")),
                w * 2,
                h * 2,
                &scaled(*w, *h, rgba, 2),
            );
        };
        let closeup = |name: &str, (w, h, rgba): &(u32, u32, Vec<u8>), area| {
            let (cw, ch, part) = crop(*w, *h, rgba, area);
            save_png(
                &out.join(format!("{label}-closeup-{name}-4x.png")),
                cw * 4,
                ch * 4,
                &scaled(cw, ch, &part, 4),
            );
        };
        let panel = |game: &Game| {
            game.world()
                .resource::<MenuView>()
                .drawn()
                .expect("a page is drawn")
                .panel
        };
        // The close-ups of a panel: its four corners, the tabs on its top
        // and bottom rings, both ends of `bar_row`, and the given extras.
        let closeups = |name: &str,
                        picture: &(u32, u32, Vec<u8>),
                        rect: super::super::geometry::CellRect,
                        bar_row: Option<u32>,
                        extra: &[(&str, (i64, i64, u32, u32))]| {
            let (x0, y0, x1, y1) = rect.pixels();
            let (x0, y0, x1, y1) = (i64::from(x0), i64::from(y0), i64::from(x1), i64::from(y1));
            let mid = (x0 + x1) / 2;
            let mut areas = vec![
                ("corner-top-left", (x0 - 8, y0 - 8, 40, 32)),
                ("corner-top-right", (x1 - 31, y0 - 8, 40, 32)),
                ("corner-bottom-left", (x0 - 8, y1 - 23, 40, 32)),
                ("corner-bottom-right", (x1 - 31, y1 - 23, 40, 32)),
                ("title-tab", (mid - 64, y0 - 4, 128, 20)),
                ("footer-tab", (mid - 56, y1 - 15, 112, 20)),
            ];
            if let Some(row) = bar_row {
                let y = i64::from(row * 8);
                areas.push(("bar-left", (x0, y - 8, 48, 24)));
                areas.push(("bar-right", (x1 - 47, y - 8, 48, 24)));
            }
            areas.extend(extra.iter().copied());
            for (part, area) in areas {
                closeup(&format!("{name}-{part}"), picture, area);
            }
        };

        // A first save, so the pages have a real thumbnail, then the top
        // menu with Save and Load and the last-save line.
        game.save_new("Road west");
        // Past the close sweep (200 ms), within the toast's 2 s.
        game.idle(16);
        let toast = window_shot(&mut game.shell);
        save_png(
            &out.join(format!("{label}-window-toast-saved.png")),
            toast.0,
            toast.1,
            &toast.2,
        );
        game.walk();
        game.walk();
        game.idle(4);
        game.tap(KeyCode::F12);
        game.idle(12);
        let top = composed(&mut game.shell);
        shot("top", &top);
        closeups("top", &top, panel(&game), None, &[]);

        // Save: the page, the name field, writing over.
        game.tap(KeyCode::ArrowDown);
        game.tap(KeyCode::Enter);
        let save = composed(&mut game.shell);
        shot("save", &save);
        let rect = panel(&game);
        closeups(
            "save",
            &save,
            rect,
            Some(rect.row + 2),
            &[(
                "junction-left",
                (
                    i64::from(rect.col * 8) - 4,
                    i64::from((rect.bottom() - 5) * 8) - 4,
                    32,
                    32,
                ),
            )],
        );
        game.tap(KeyCode::Enter);
        game.type_name("Morning walk");
        let name = composed(&mut game.shell);
        shot("save-name", &name);
        let rect = panel(&game);
        closeups(
            "save-name",
            &name,
            rect,
            None,
            &[(
                "input",
                (
                    i64::from(rect.col * 8),
                    i64::from((rect.row + 3) * 8) - 8,
                    128,
                    32,
                ),
            )],
        );
        game.tap(KeyCode::Escape);
        game.tap(KeyCode::ArrowDown);
        game.tap(KeyCode::ArrowDown);
        game.tap(KeyCode::Enter);
        let over = composed(&mut game.shell);
        shot("save-overwrite", &over);
        let rect = panel(&game);
        closeups(
            "save-overwrite",
            &over,
            rect,
            None,
            &[(
                "buttons",
                (
                    i64::from(rect.col * 8) + 48,
                    i64::from((rect.row + 4) * 8) - 8,
                    160,
                    24,
                ),
            )],
        );
        game.tap(KeyCode::ArrowLeft);
        game.tap(KeyCode::Enter);
        game.finish_save();
        game.idle(16);
        let toast = window_shot(&mut game.shell);
        save_png(
            &out.join(format!("{label}-window-toast-overwritten.png")),
            toast.0,
            toast.1,
            &toast.2,
        );
        host.add("Old game", true);
        host.add("From DOS", false);
        {
            let mut slots = host.slots.lock().unwrap();
            let import = slots
                .iter_mut()
                .find(|(i, _)| i.name == "From DOS")
                .unwrap();
            import.0.imported = true;
            import.0.turn = 0;
            import.0.clock = [139, 3, 29, 18, 2];
            import.0.place = "Britain".into();
        }
        {
            let mut slots = host.slots.lock().unwrap();
            let mut cheats = slots[0].clone();
            cheats.0.id = SlotId("s0009".into());
            cheats.0.name = "With a cheat".into();
            cheats.0.modified = true;
            cheats.0.saved_at_unix = 1;
            slots.push(cheats);
        }

        // Load: the real slot, the damaged one, the import, the cheat mark;
        // rename, delete and "Loading...".
        game.idle(4);
        game.tap(KeyCode::F12);
        game.idle(12);
        game.tap(KeyCode::ArrowDown);
        game.tap(KeyCode::ArrowDown);
        game.tap(KeyCode::Enter);
        let names: Vec<String> = game
            .world()
            .resource::<SlotListing>()
            .slots
            .iter()
            .map(|slot| slot.info.name.clone())
            .collect();
        eprintln!("{label}: listed {names:?}");
        for (at, name) in names.iter().enumerate() {
            let load = composed(&mut game.shell);
            let what = name.to_lowercase().replace(' ', "-");
            shot(&format!("load-{at}-{what}"), &load);
            let rect = panel(&game);
            let split = i64::from((rect.col + super::super::render_slots::LOAD_SPLIT) * 8);
            let bottom = i64::from((rect.bottom() - 3) * 8);
            closeups(
                &format!("load-{at}"),
                &load,
                rect,
                Some(rect.row + 1 + at as u32),
                &[
                    (
                        "split-top",
                        (split - 12, i64::from(rect.row * 8) - 4, 32, 32),
                    ),
                    ("split-bottom", (split - 12, bottom - 12, 32, 32)),
                    (
                        "junction-left",
                        (i64::from(rect.col * 8) - 4, bottom - 12, 32, 32),
                    ),
                    (
                        "junction-right",
                        (i64::from(rect.right() * 8) - 20, bottom - 12, 32, 32),
                    ),
                    (
                        "thumbnail",
                        (split + 8, i64::from(rect.row * 8) + 8, 144, 80),
                    ),
                    (
                        "details",
                        (split + 8, i64::from(rect.row * 8) + 72, 144, 64),
                    ),
                ],
            );
            game.tap(KeyCode::ArrowDown);
        }
        let road = names.iter().position(|n| n == "Road west").unwrap();
        for _ in 0..names.len() {
            game.tap(KeyCode::ArrowUp);
        }
        for _ in 0..road {
            game.tap(KeyCode::ArrowDown);
        }
        game.tap(KeyCode::F2);
        let rename = composed(&mut game.shell);
        shot("load-rename", &rename);
        closeups("load-rename", &rename, panel(&game), None, &[]);
        game.tap(KeyCode::Escape);
        game.tap(KeyCode::Delete);
        let delete = composed(&mut game.shell);
        shot("load-delete", &delete);
        closeups("load-delete", &delete, panel(&game), None, &[]);
        game.tap(KeyCode::Escape);
        // "Loading..." is drawn for one frame before the restore: the
        // picture of that frame.
        let press = |state| TimedKey {
            key: KeyCode::Enter,
            state,
            repeat: false,
            owner: Owner::Game,
        };
        game.step(&[press(ButtonState::Pressed)]);
        let loading = composed(&mut game.shell);
        shot("load-loading", &loading);
        closeups("load-loading", &loading, panel(&game), None, &[]);
        game.step(&[press(ButtonState::Released)]);
        assert!(game.app_exit(), "the load ends the app");
        eprintln!("{label}: written");
        let _ = fs::remove_dir_all(&game_dir);
    }
}

/// The menu's no-spoiler suite (M4-14), on this fake host.
#[path = "spoiler_tests.rs"]
mod spoiler;

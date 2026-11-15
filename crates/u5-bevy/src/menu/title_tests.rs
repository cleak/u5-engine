//! The title layer before play and Return to the title (design 3.2.4,
//! 3.2.5, D-M4.3; plan M4-12), headless over a synthetic intro waiting at
//! its title menu: where the menu key opens the layer and where it falls
//! through, that no key reaches the intro while it is open and every key
//! does while it is shut, Continue and Load through the game's own Journey
//! Onward (with a slot, without one, with an empty one, with a failing
//! host), the idle Return-to-View with the layer open, the drawn page and
//! the relaunch Return to the title asks for.
//!
//! The intro's own driver needs the game's pictures, so a probe stands in
//! its slot: it records the keys the intro would read this frame, as
//! `drive_visual_intro` reads them, and runs one menu poll pass when there
//! were none.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};

use bevy::asset::AssetPlugin;
use bevy::ecs::event::EventCursor;
use bevy::input::ButtonState;
use bevy::input::InputPlugin;
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::time::TimeUpdateStrategy;
use u5_runtime::enhancements::Toggle;
use u5_runtime::test_fixtures::{debug_game_dir, saved_game_seed_bytes, write_empty_ool_mirrors};
use u5_runtime::{CH_FONT_LEN, IBM_CH_FILE, SAVE_AVATAR_NAME_OFFSET, SAVED_GAM_FILENAME};

use super::*;
use crate::enh::equivalence::FRAME;
use crate::enh::input::KeyLatch;
use crate::enh::synthetic::SyntheticInputOwner;
use crate::enh::toast::Toasts;
use crate::enh::{EnhSet, ShellExitSlot, VisualLaunchConfig, add_enhancements};
use crate::menu::geometry::MenuPicture;
use crate::menu::host::{MenuHost, SlotFailure, SlotId, SlotInfo, StoreSlot};
use crate::menu::pages::MenuCommand;
use crate::menu::pages::{
    Dialog, MenuModel, Page, PageState, Row, RowAction, ShellSnapshot, SlotEntry, page_rows,
};
use crate::menu::render::{MenuView, render_page};
use crate::menu::{
    MenuCommandEvent, MenuPlugin, MenuState, quit_to_title, quit_to_title_exit, text as t,
    world_snapshot,
};
use crate::{Relaunch, ShellExit};
use crate::{
    VisualIntroPanel, VisualIntroState, keys_pressed_in_typed_order, title_menu_intro_for_tests,
};

// --- the harness -------------------------------------------------------------

/// What the intro would have read, frame by frame.
#[derive(Resource, Default)]
struct IntroKeys(Vec<Vec<KeyCode>>);

/// Stands in the intro driver's slot: records the keys it would read and,
/// on a frame without one, runs a menu poll pass (the idle count the
/// Return-to-View preview waits for).
fn probe_intro(
    keyboard: Res<ButtonInput<KeyCode>>,
    events: Option<Res<Events<KeyboardInput>>>,
    mut cursor: Local<EventCursor<KeyboardInput>>,
    mut seen: ResMut<IntroKeys>,
    intro: Option<ResMut<VisualIntroState>>,
) {
    let keys = keys_pressed_in_typed_order(&keyboard, events.as_deref(), &mut cursor);
    if let Some(mut intro) = intro
        && matches!(intro.panel, VisualIntroPanel::Menu)
    {
        if keys.is_empty() {
            intro.menu_idle_ticks = intro.menu_idle_ticks.saturating_add(1);
        } else {
            intro.menu_idle_ticks = 0;
        }
    }
    seen.0.push(keys);
}

fn launch_with(toggles: &[Toggle]) -> VisualLaunchConfig {
    let mut launch = VisualLaunchConfig::classic();
    for toggle in toggles {
        launch.enhancements.toggles.set(*toggle, true);
    }
    launch
}

fn blank_font() -> u5_runtime::FixedCellFont {
    u5_runtime::parse_ch_font(&vec![0u8; CH_FONT_LEN], IBM_CH_FILE).unwrap()
}

/// A working save in `dir`: the seed (no Avatar), or with an Avatar's
/// name, which is what Journey Onward checks.
fn write_save(dir: &Path, avatar: bool) -> Vec<u8> {
    let mut bytes = saved_game_seed_bytes(0x11, 0, 5, 5);
    if avatar {
        bytes[SAVE_AVATAR_NAME_OFFSET] = b'A';
    }
    fs::write(dir.join(SAVED_GAM_FILENAME), &bytes).unwrap();
    write_empty_ool_mirrors(dir);
    fs::write(
        dir.join(u5_runtime::SAVED_OOL_FILENAME),
        vec![0; u5_runtime::SAVED_OOL_LEN],
    )
    .unwrap();
    bytes
}

/// The default player app at its title menu, headless.
struct Title {
    app: App,
    dir: PathBuf,
    next: Vec<(KeyCode, ButtonState)>,
}

impl Title {
    fn new(launch: &VisualLaunchConfig, intro: impl FnOnce(&mut VisualIntroState)) -> Self {
        let dir = debug_game_dir();
        write_save(&dir, true);
        Self::in_dir(dir, launch, intro)
    }

    fn in_dir(
        dir: PathBuf,
        launch: &VisualLaunchConfig,
        edit: impl FnOnce(&mut VisualIntroState),
    ) -> Self {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default(), InputPlugin))
            .init_asset::<Image>()
            .insert_resource(TimeUpdateStrategy::ManualDuration(FRAME));
        let mut intro = title_menu_intro_for_tests(dir.clone());
        edit(&mut intro);
        app.insert_resource(intro)
            .init_resource::<IntroKeys>()
            .add_systems(
                Update,
                probe_intro.after(EnhSet::Feed).before(EnhSet::Observe),
            );
        add_enhancements(&mut app, launch);
        // The intro chain puts the observers before the presenters; here
        // there is no chain.
        app.configure_sets(Update, EnhSet::Present.after(EnhSet::Observe));
        if launch.wants_enhancements() {
            app.insert_resource(ShellExitSlot::default());
        }
        // The game's font is not in the scratch folder.
        if app.is_plugin_added::<MenuPlugin>() {
            app.insert_resource(TitleFont(blank_font()));
        }
        let mut title = Self {
            app,
            dir,
            next: Vec::new(),
        };
        title.frames(3);
        title
    }

    fn frame(&mut self) -> &mut Self {
        for (key, state) in std::mem::take(&mut self.next) {
            self.app.world_mut().send_event(KeyboardInput {
                key_code: key,
                logical_key: Key::Unidentified(NativeKey::Unidentified),
                state,
                text: None,
                repeat: false,
                window: Entity::PLACEHOLDER,
            });
        }
        self.app.update();
        self
    }

    fn frames(&mut self, count: usize) -> &mut Self {
        for _ in 0..count {
            self.frame();
        }
        self
    }

    /// Presses `key` for one frame and releases it in the next.
    fn tap(&mut self, key: KeyCode) -> &mut Self {
        self.next.push((key, ButtonState::Pressed));
        self.frame();
        self.next.push((key, ButtonState::Released));
        self.frame()
    }

    fn world(&self) -> &World {
        self.app.world()
    }

    fn intro(&self) -> &VisualIntroState {
        self.world().resource::<VisualIntroState>()
    }

    fn intro_mut(&mut self) -> Mut<'_, VisualIntroState> {
        self.app.world_mut().resource_mut::<VisualIntroState>()
    }

    fn page(&self) -> Option<PageState> {
        self.world()
            .resource::<MenuState>()
            .model()
            .map(MenuModel::current)
    }

    fn open(&self) -> bool {
        self.world()
            .resource::<MenuState>()
            .model()
            .is_some_and(MenuModel::is_title)
    }

    fn modal(&self) -> bool {
        self.world().resource::<KeyLatch>().is_modal()
    }

    /// Every key the intro read since the last call.
    fn intro_keys(&mut self) -> Vec<KeyCode> {
        let mut seen = self.app.world_mut().resource_mut::<IntroKeys>();
        std::mem::take(&mut seen.0).into_iter().flatten().collect()
    }

    fn launched(&self) -> bool {
        self.intro().launch_result.lock().unwrap().is_some()
    }

    fn toast(&self) -> Option<String> {
        self.world()
            .get_resource::<Toasts>()
            .and_then(|toasts| toasts.current().map(str::to_string))
    }
}

impl Drop for Title {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn title_page(cursor: usize) -> Option<PageState> {
    Some(PageState {
        page: Page::Title,
        cursor,
    })
}

const LAYER: [Toggle; 1] = [Toggle::MenuTitleLayer];

// --- a slot host --------------------------------------------------------------

/// Slots in memory, each the working save it restores.
struct Slots {
    dir: PathBuf,
    slots: Mutex<Vec<(SlotInfo, Vec<u8>)>>,
    fail_restore: Option<SlotFailure>,
    restores: Mutex<Vec<SlotId>>,
}

impl Slots {
    fn new(dir: &Path, slots: Vec<(SlotInfo, Vec<u8>)>) -> Arc<Self> {
        Arc::new(Self {
            dir: dir.to_path_buf(),
            slots: Mutex::new(slots),
            fail_restore: None,
            restores: Mutex::new(Vec::new()),
        })
    }

    fn restores(&self) -> Vec<SlotId> {
        self.restores.lock().unwrap().clone()
    }
}

fn info(id: &str, name: &str, saved_at_unix: u64) -> SlotInfo {
    SlotInfo {
        id: SlotId(id.to_string()),
        name: name.to_string(),
        avatar: "Ann".to_string(),
        place: "Britannia".to_string(),
        clock: [139, 4, 6, 9, 15],
        turn: 7,
        saved_at_unix,
        ..SlotInfo::default()
    }
}

/// A save image with or without an Avatar, and its Avatar byte as a mark.
fn save_image(avatar: Option<u8>) -> Vec<u8> {
    let mut bytes = saved_game_seed_bytes(0x11, 0, 5, 5);
    if let Some(mark) = avatar {
        bytes[SAVE_AVATAR_NAME_OFFSET] = mark;
    }
    bytes
}

impl MenuHost for Slots {
    fn list_slots(&self) -> Result<Vec<SlotInfo>, SlotFailure> {
        let mut slots: Vec<SlotInfo> = self
            .slots
            .lock()
            .unwrap()
            .iter()
            .map(|(info, _)| info.clone())
            .collect();
        slots.sort_by_key(|slot| std::cmp::Reverse(slot.saved_at_unix));
        Ok(slots)
    }

    fn store_slot(&self, _request: StoreSlot) -> Receiver<Result<SlotInfo, SlotFailure>> {
        let (send, reply) = mpsc::channel();
        let _ = send.send(Err(SlotFailure::Reason("not at the title".to_string())));
        reply
    }

    fn restore_slot(&self, id: &SlotId) -> Result<(), SlotFailure> {
        self.restores.lock().unwrap().push(id.clone());
        if let Some(failure) = self.fail_restore.clone() {
            return Err(failure);
        }
        let slots = self.slots.lock().unwrap();
        let (_, bytes) = slots
            .iter()
            .find(|(info, _)| info.id == *id)
            .ok_or_else(|| SlotFailure::Reason("no such slot".to_string()))?;
        fs::write(self.dir.join(SAVED_GAM_FILENAME), bytes).unwrap();
        Ok(())
    }

    fn roll_back_restore(&self) -> Result<(), SlotFailure> {
        Ok(())
    }

    fn rename_slot(&self, _id: &SlotId, _name: &str) -> Result<(), SlotFailure> {
        Ok(())
    }

    fn delete_slot(&self, _id: &SlotId) -> Result<(), SlotFailure> {
        Ok(())
    }
}

fn slots_launch(host: Arc<Slots>) -> VisualLaunchConfig {
    let mut launch = launch_with(&[Toggle::MenuTitleLayer, Toggle::MenuNamedSlots]);
    launch.menu_host = Some(host);
    launch
}

// --- the pages ------------------------------------------------------------------

fn entry(info: SlotInfo) -> SlotEntry {
    SlotEntry { info, thumb: None }
}

#[test]
fn the_layer_s_rows_with_and_without_slots() {
    let labels = |snap: &ShellSnapshot| -> Vec<(&'static str, RowAction)> {
        page_rows(Page::Title, snap)
            .iter()
            .map(|row: &Row| (row.label, row.action))
            .collect()
    };
    let without = ShellSnapshot {
        intro_shell: true,
        ..ShellSnapshot::default()
    };
    assert_eq!(
        labels(&without),
        [
            (t::CONTINUE, RowAction::Continue(None)),
            (t::SETTINGS, RowAction::Settings),
            (t::QUIT, RowAction::QuitToDesktop),
        ]
    );
    let mut damaged = info("s3", "Broken", 900);
    damaged.damaged = Some("SAVED.GAM is missing".to_string());
    let with = ShellSnapshot {
        intro_shell: true,
        slots: true,
        slot_list: Arc::new(vec![
            entry(damaged),
            entry(info("s2", "Second try", 500)),
            entry(info("s1", "First", 400)),
        ]),
        ..ShellSnapshot::default()
    };
    assert_eq!(
        labels(&with),
        [
            (t::CONTINUE, RowAction::Continue(Some(1))),
            (t::LOAD, RowAction::Load),
            (t::SETTINGS, RowAction::Settings),
            (t::QUIT, RowAction::QuitToDesktop),
        ],
        "Continue loads the newest slot that can be loaded"
    );
    let empty = ShellSnapshot {
        slots: true,
        ..without
    };
    assert_eq!(
        labels(&empty)[0],
        (t::CONTINUE, RowAction::Continue(None)),
        "with no slot to load, Continue presses J on the working save"
    );
}

#[test]
fn the_layer_is_drawn_in_the_title_menu_s_frame() {
    let font = blank_font();
    let snap = ShellSnapshot {
        intro_shell: true,
        slots: true,
        slot_list: Arc::new(vec![entry(info("s2", "Second try", 500))]),
        ..ShellSnapshot::default()
    };
    let rows = page_rows(Page::Title, &snap);
    assert_eq!(
        crate::menu::render::title_label(&rows[0], &snap),
        t::continue_label("Second try", "139/04/06")
    );
    let drawn = render_page(
        &font,
        &MenuPicture::FRAME,
        Default::default(),
        PageState {
            page: Page::Title,
            cursor: 0,
        },
        &snap,
    );
    // The title menu's own frame: cells 0-39 x 15-24 (the M4-01 sheet).
    assert_eq!(
        drawn.panel.pixels(),
        (0, 120, 319, 199),
        "the layer sits exactly on the title menu's frame"
    );
    // Above the frame the title art shows through.
    let px = |x: u32, y: u32| {
        let i = ((y * drawn.image.w + x) * 4) as usize;
        drawn.image.rgba[i..i + 4].to_vec()
    };
    assert_eq!(px(160, 60)[3], 0, "the art above the frame is not covered");
    assert_eq!(px(160, 119)[3], 0);
    assert_eq!(px(0, 160)[3], 0xff, "the ring is opaque");
}

// --- opening and keys ----------------------------------------------------------

#[test]
fn the_menu_key_opens_the_layer_only_while_the_title_menu_polls() {
    // The flourish: the key falls through to the intro, as in 1.0.1.
    let mut title = Title::new(&launch_with(&LAYER), |intro| {
        intro.dispatch = crate::UnifiedMenuDispatch::new();
        intro.title_flourish_complete = false;
    });
    title.intro_keys();
    title.tap(KeyCode::F12);
    assert!(!title.open() && !title.modal());
    assert_eq!(title.intro_keys(), [KeyCode::F12], "F12 falls through");

    // A message waiting for its own key, and the preview still owed.
    for edit in [
        (|intro: &mut VisualIntroState| {
            intro.message = "No game".to_string();
            intro.message_waiting_for_key = true;
        }) as fn(&mut VisualIntroState),
        |intro| intro.pending_auto_return_to_view = true,
    ] {
        let mut title = Title::new(&launch_with(&LAYER), edit);
        title.intro_keys();
        title.tap(KeyCode::F12);
        assert!(!title.open());
        assert_eq!(title.intro_keys(), [KeyCode::F12]);
    }

    // The title menu polling: the layer opens on Continue and takes the
    // key, without the overlay model.
    let mut title = Title::new(&launch_with(&LAYER), |_| {});
    title.intro_keys();
    title.tap(KeyCode::F12);
    assert!(title.open() && title.modal());
    assert_eq!(title.page(), title_page(0));
    assert!(title.intro_keys().is_empty(), "the menu key is the layer's");
    assert!(
        title
            .world()
            .get_resource::<crate::enh::overlay::OverlayState>()
            .is_none_or(|overlay| overlay.open().is_none()),
        "the title layer never touches the overlay model"
    );
}

#[test]
fn the_title_s_menu_and_keys_are_unchanged_with_the_layer_shut() {
    let keys = [
        KeyCode::ArrowDown,
        KeyCode::ArrowUp,
        KeyCode::KeyX,
        KeyCode::Space,
        KeyCode::Escape,
        KeyCode::Enter,
        KeyCode::KeyR,
    ];
    let run = |launch: &VisualLaunchConfig| {
        let mut title = Title::new(launch, |_| {});
        title.intro_keys();
        for key in keys {
            title.tap(key);
        }
        let seen = title.intro_keys();
        let idle = title.intro().menu_idle_ticks;
        (seen, idle)
    };
    let classic = run(&VisualLaunchConfig::classic());
    assert_eq!(classic.0, keys);
    for launch in [
        launch_with(&LAYER),
        launch_with(&[Toggle::MenuTitleLayer, Toggle::MenuOverlay]),
    ] {
        assert_eq!(run(&launch), classic);
    }
}

#[test]
fn no_key_reaches_the_intro_while_the_layer_is_open() {
    let mut title = Title::new(&launch_with(&LAYER), |_| {});
    title.tap(KeyCode::F12);
    title.intro_keys();
    // Menu keys and the title menu's own letters alike.
    for key in [
        KeyCode::ArrowDown,
        KeyCode::KeyJ,
        KeyCode::KeyC,
        KeyCode::Space,
        KeyCode::ArrowLeft,
        KeyCode::ArrowUp,
    ] {
        title.tap(key);
    }
    assert!(title.open());
    assert!(title.intro_keys().is_empty());
    assert!(!title.launched());
}

#[test]
fn escape_and_f12_close_the_layer_and_hand_the_keys_back() {
    for close in [KeyCode::Escape, KeyCode::F12] {
        let mut title = Title::new(&launch_with(&LAYER), |_| {});
        title.tap(KeyCode::F12);
        title.tap(KeyCode::ArrowDown);
        assert_eq!(title.page(), title_page(1));
        title.tap(close);
        assert!(!title.open() && !title.modal());
        assert!(
            title.world().resource::<MenuView>().sweep.visible(),
            "the layer sweeps out"
        );
        title.intro_keys();
        title.tap(KeyCode::ArrowDown);
        assert_eq!(title.intro_keys(), [KeyCode::ArrowDown]);
        title.frames(20);
        assert!(!title.world().resource::<MenuView>().sweep.visible());
    }
    // Settings and back again stays on the layer.
    let mut title = Title::new(&launch_with(&LAYER), |_| {});
    title
        .tap(KeyCode::F12)
        .tap(KeyCode::ArrowDown)
        .tap(KeyCode::Enter);
    assert!(matches!(
        title.page().map(|page| page.page),
        Some(Page::Settings(_))
    ));
    title.tap(KeyCode::Escape);
    assert_eq!(title.page(), title_page(1));
}

#[test]
fn quit_at_the_title_leaves_the_game() {
    let mut title = Title::new(&launch_with(&LAYER), |_| {});
    title
        .tap(KeyCode::F12)
        .tap(KeyCode::ArrowDown)
        .tap(KeyCode::ArrowDown);
    title.next.push((KeyCode::Enter, ButtonState::Pressed));
    title.frame();
    let exits = title
        .world()
        .resource::<Events<AppExit>>()
        .iter_current_update_events()
        .count();
    assert_eq!(exits, 1);
    assert_eq!(
        title.world().resource::<ShellExitSlot>().take(),
        ShellExit::default(),
        "leaving the game asks for no relaunch"
    );
}

// --- Continue and Load -----------------------------------------------------------

#[test]
fn continue_without_slots_presses_j_on_the_working_save() {
    let mut title = Title::new(&launch_with(&LAYER), |_| {});
    title.tap(KeyCode::F12);
    title.intro_keys();
    title.tap(KeyCode::Enter);
    // The game's own Journey Onward read the working save and handed it to
    // gameplay: `attempt_visual_journey_onward_load` is the one place that
    // fills the hand-off, and `transition_visual_intro_to_gameplay`, which
    // takes it, inserts the emulator-profile load latency.
    assert!(title.launched());
    assert!(!title.open() && !title.modal());
    assert!(
        !title.world().resource::<MenuView>().sweep.visible(),
        "the game takes the picture at once"
    );
    assert!(title.intro_keys().is_empty(), "the J was typed, not leaked");
    let owner = title.world().resource::<SyntheticInputOwner>();
    assert_eq!(owner.holder(), None, "the TitleJ token is released");
    assert_eq!(owner.queued(), 0);
}

#[test]
fn continue_on_an_empty_working_save_shows_the_game_s_message() {
    let dir = debug_game_dir();
    write_save(&dir, false);
    let mut title = Title::in_dir(dir, &launch_with(&LAYER), |_| {});
    title.tap(KeyCode::F12).tap(KeyCode::Enter);
    assert!(!title.launched());
    assert!(title.intro().message_waiting_for_key);
    assert_eq!(
        title.intro().message,
        crate::visual_intro_empty_save_message()
    );
    assert!(!title.open());
    // The message waits for its own key: the menu key falls through.
    title.intro_keys();
    title.tap(KeyCode::F12);
    assert!(!title.open());
    assert_eq!(title.intro_keys(), [KeyCode::F12]);
}

#[test]
fn continue_restores_the_newest_slot_then_presses_j() {
    let dir = debug_game_dir();
    write_save(&dir, false);
    let host = Slots::new(
        &dir,
        vec![
            (info("s1", "Older", 400), save_image(Some(b'O'))),
            (info("s2", "Newer", 500), save_image(Some(b'N'))),
        ],
    );
    let mut title = Title::in_dir(dir.clone(), &slots_launch(host.clone()), |_| {});
    title.tap(KeyCode::F12);
    assert_eq!(title.page(), title_page(0));
    let snap = world_snapshot(title.world());
    let rows = page_rows(Page::Title, &snap);
    assert_eq!(rows[0].action, RowAction::Continue(Some(0)));
    assert_eq!(
        crate::menu::render::title_label(&rows[0], &snap),
        t::continue_label("Newer", "139/04/06")
    );
    title.next.push((KeyCode::Enter, ButtonState::Pressed));
    title.frame();
    assert_eq!(
        title.page().map(|page| page.page),
        Some(Page::Dialog(Dialog::Loading(0))),
        "Loading... shows for a frame"
    );
    assert!(host.restores().is_empty());
    title.next.push((KeyCode::Enter, ButtonState::Released));
    title.frame();
    assert_eq!(host.restores(), [SlotId("s2".to_string())]);
    let working = fs::read(dir.join(SAVED_GAM_FILENAME)).unwrap();
    assert_eq!(working[SAVE_AVATAR_NAME_OFFSET], b'N');
    assert!(title.launched(), "the game's Journey Onward loaded it");
    assert!(!title.open() && !title.modal());
}

#[test]
fn load_at_the_title_restores_the_chosen_slot_then_presses_j() {
    let dir = debug_game_dir();
    write_save(&dir, false);
    let host = Slots::new(
        &dir,
        vec![
            (info("s1", "Older", 400), save_image(Some(b'O'))),
            (info("s2", "Newer", 500), save_image(Some(b'N'))),
        ],
    );
    let mut title = Title::in_dir(dir.clone(), &slots_launch(host.clone()), |_| {});
    title
        .tap(KeyCode::F12)
        .tap(KeyCode::ArrowDown)
        .tap(KeyCode::Enter);
    assert_eq!(title.page().map(|page| page.page), Some(Page::Load));
    title.tap(KeyCode::ArrowDown).tap(KeyCode::Enter).frame();
    assert_eq!(host.restores(), [SlotId("s1".to_string())]);
    let working = fs::read(dir.join(SAVED_GAM_FILENAME)).unwrap();
    assert_eq!(working[SAVE_AVATAR_NAME_OFFSET], b'O');
    assert!(title.launched());
    assert!(!title.open());
}

#[test]
fn an_avatar_less_slot_shows_the_empty_save_message() {
    let dir = debug_game_dir();
    write_save(&dir, true);
    let host = Slots::new(&dir, vec![(info("s1", "Empty", 400), save_image(None))]);
    let mut title = Title::in_dir(dir, &slots_launch(host.clone()), |_| {});
    title.tap(KeyCode::F12).tap(KeyCode::Enter).frame();
    assert_eq!(host.restores().len(), 1);
    assert!(!title.launched());
    assert!(title.intro().message_waiting_for_key);
    assert_eq!(
        title.intro().message,
        crate::visual_intro_empty_save_message()
    );
    assert!(!title.open() && !title.modal());
}

#[test]
fn a_failed_restore_at_the_title_says_why_and_presses_nothing() {
    let dir = debug_game_dir();
    write_save(&dir, true);
    let mut host = Slots::new(
        &dir,
        vec![(info("s1", "Kept", 400), save_image(Some(b'K')))],
    );
    Arc::get_mut(&mut host).unwrap().fail_restore = Some(SlotFailure::Reason("disk gone".into()));
    let mut title = Title::in_dir(dir, &slots_launch(host.clone()), |_| {});
    title.tap(KeyCode::F12).tap(KeyCode::Enter).frame();
    assert_eq!(host.restores().len(), 1);
    assert!(!title.launched());
    assert_eq!(
        title.toast().as_deref(),
        Some(t::fill(t::LOAD_FAILED, &["Kept", "disk gone"]).as_str())
    );
    assert_eq!(title.page(), title_page(0), "back on the layer");
    assert!(title.modal());
}

// --- the idle preview ----------------------------------------------------------

#[test]
fn the_idle_return_to_view_still_fires_with_the_layer_open() {
    let mut title = Title::new(&launch_with(&LAYER), |_| {});
    title.tap(KeyCode::F12);
    let before = title.intro().menu_idle_ticks;
    // Keys in the layer are not keys at the title menu: its idle count
    // runs on.
    for _ in 0..5 {
        title.tap(KeyCode::ArrowDown).tap(KeyCode::ArrowUp);
    }
    assert!(title.open());
    assert!(title.intro().menu_idle_ticks >= before + 20);
    // The preview fires (its pictures need the game's files, so the panel
    // is put in place as the subflow puts it): the layer goes that frame.
    title.intro_mut().panel = VisualIntroPanel::ReturnToView {
        preview_frames: Vec::new(),
        frame_metadata: Vec::new(),
        preview_frame_index: 0,
    };
    title.frame();
    assert!(!title.open() && !title.modal());
    assert!(!title.world().resource::<MenuView>().sweep.visible());
    title.intro_keys();
    title.tap(KeyCode::Space);
    assert_eq!(
        title.intro_keys(),
        [KeyCode::Space],
        "the preview gets its key"
    );
}

// --- Return to the title -----------------------------------------------------------

#[test]
fn return_to_the_title_asks_for_a_relaunch_with_no_boot_key() {
    assert_eq!(
        quit_to_title_exit(),
        ShellExit {
            relaunch: Some(Relaunch { boot_key: None })
        }
    );
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_event::<MenuCommandEvent>()
        .add_event::<AppExit>()
        .insert_resource(ShellExitSlot::default())
        .add_systems(Update, quit_to_title);
    app.world_mut()
        .send_event(MenuCommandEvent(MenuCommand::QuitToTitle));
    app.update();
    assert_eq!(
        app.world()
            .resource::<Events<AppExit>>()
            .iter_current_update_events()
            .count(),
        1
    );
    assert_eq!(
        app.world().resource::<ShellExitSlot>().take(),
        quit_to_title_exit()
    );
}

// --- the render check -------------------------------------------------------------

/// The M4-12 render check: the title layer in the real default-player app
/// over the real title menu, read back from the app's own images (the
/// intro's picture under the backdrop under the menu node), for the user
/// to hold beside the M4-01 sheet. Review output only, never committed,
/// and `#[ignore]`d:
///
/// ```text
/// U5_MOCKUP_PROFILE=<copy> U5_MENU_SHOTS=<folder> \
///   cargo test -p u5-bevy --lib menu::title::tests::title_layer_review_shots -- --ignored
/// ```
///
/// For a 4:3 window and a 16:9 grow-left one, with and without slots: the
/// title before, the open sweep, the layer, each row selected, the Load
/// and Settings pages from the title, the close sweep, 4x close-ups of the
/// corners, tabs and bar, and the whole window as the nodes place it.
#[test]
#[ignore = "review output: needs U5_MOCKUP_PROFILE and U5_MENU_SHOTS"]
fn title_layer_review_shots() {
    use bevy::window::WindowResolution;
    use u5_runtime::enhancements::{
        CliSetting, HostCapabilities, LockState, ResolveInput, resolve,
    };
    use u5_runtime::test_fixtures::{configured_test_dir, copy_asset_writable, unique_scratch_dir};

    use crate::menu::frame::{CLEAR, DIM, MenuImage, composite_over};
    use crate::menu::render::{MenuBackdrop, MenuNode, frame_rect};
    use crate::menu::review_tests::{crop, save_png, scaled};

    let (Some(profile), Some(out)) = (
        configured_test_dir("U5_MOCKUP_PROFILE"),
        configured_test_dir("U5_MENU_SHOTS"),
    ) else {
        eprintln!("skipped: set U5_MOCKUP_PROFILE and U5_MENU_SHOTS");
        return;
    };
    fs::create_dir_all(&out).unwrap();

    struct Case {
        label: &'static str,
        window: (u32, u32),
        grow: bool,
        slots: bool,
    }
    let cases = [
        Case {
            label: "43-1600x1200-slots",
            window: (1600, 1200),
            grow: false,
            slots: true,
        },
        Case {
            label: "43-1600x1200-no-slots",
            window: (1600, 1200),
            grow: false,
            slots: false,
        },
        Case {
            label: "169-2560x1440-slots",
            window: (2560, 1440),
            grow: true,
            slots: true,
        },
        Case {
            label: "169-1920x1080-no-slots",
            window: (1920, 1080),
            grow: true,
            slots: false,
        },
    ];

    /// The picture the frame sprite shows: the presented one, or the
    /// intro's own.
    fn shown_handle(world: &World) -> Handle<Image> {
        world
            .get_resource::<crate::enh::present::PresentHost>()
            .and_then(crate::enh::present::PresentHost::presented)
            .cloned()
            .unwrap_or_else(|| {
                world
                    .resource::<VisualIntroState>()
                    .image_handle
                    .clone()
                    .unwrap()
            })
    }

    /// The intro's picture, the backdrop's dim and the menu node over it.
    fn composed(world: &mut World) -> (u32, u32, Vec<u8>) {
        let handle = shown_handle(world);
        let image = world.resource::<Assets<Image>>().get(&handle).unwrap();
        let (size, mut out) = (image.size(), image.data.clone().unwrap());
        let mut backdrops = world.query_filtered::<(), With<MenuBackdrop>>();
        if backdrops.iter(world).next().is_some() {
            let mut dim = MenuImage::new(size.x, size.y);
            dim.rgba.chunks_mut(4).for_each(|p| p.copy_from_slice(&DIM));
            out = composite_over(&out, &dim);
        }
        let mut nodes = world.query_filtered::<&ImageNode, With<MenuNode>>();
        if let Some(node) = nodes.iter(world).next() {
            let image = world.resource::<Assets<Image>>().get(&node.image).unwrap();
            assert_eq!(
                image.size(),
                size,
                "the layer is the title's picture's size"
            );
            out = composite_over(
                &out,
                &MenuImage {
                    w: size.x,
                    h: size.y,
                    rgba: image.data.clone().unwrap(),
                },
            );
        }
        (size.x, size.y, out)
    }

    /// The whole window: black, the intro sprite sampled nearest over its
    /// rectangle, the backdrop over everything and the menu node over the
    /// sprite's rectangle.
    fn window_shot(world: &mut World) -> (u32, u32, Vec<u8>) {
        let handle = shown_handle(world);
        let image = world.resource::<Assets<Image>>().get(&handle).unwrap();
        let (size, game) = (image.size(), image.data.clone().unwrap());
        let window = world.query::<&Window>().iter(world).next().unwrap().clone();
        let (sprite, transform) = world
            .query::<(&Sprite, &Transform)>()
            .iter(world)
            .find(|(sprite, _)| sprite.image == handle)
            .map(|(s, t)| (s.clone(), *t))
            .unwrap();
        let rect = frame_rect(&window, &sprite, &transform).unwrap();
        let px = |v: Val| match v {
            Val::Px(px) => px,
            _ => panic!("the node is placed in pixels"),
        };
        let mut nodes = world.query_filtered::<(&Node, &ImageNode), With<MenuNode>>();
        let menu = nodes.iter(world).next().map(|(node, image)| {
            let r = Rect::new(
                px(node.left),
                px(node.top),
                px(node.left) + px(node.width),
                px(node.top) + px(node.height),
            );
            let image = world.resource::<Assets<Image>>().get(&image.image).unwrap();
            (r, image.data.clone().unwrap())
        });
        let mut backdrops = world.query_filtered::<(), With<MenuBackdrop>>();
        let backdrop = backdrops.iter(world).next().is_some();
        let (ww, wh) = (
            window.resolution.physical_width(),
            window.resolution.physical_height(),
        );
        let scale = window.resolution.scale_factor();
        let sample = |r: Rect, rgba: &[u8], x: u32, y: u32| -> Option<[u8; 4]> {
            let (lx, ly) = ((x as f32 + 0.5) / scale, (y as f32 + 0.5) / scale);
            if !r.contains(Vec2::new(lx, ly)) {
                return None;
            }
            let u = (((lx - r.min.x) / r.width()) * size.x as f32) as u32;
            let v = (((ly - r.min.y) / r.height()) * size.y as f32) as u32;
            let i = ((v.min(size.y - 1) * size.x + u.min(size.x - 1)) * 4) as usize;
            Some([rgba[i], rgba[i + 1], rgba[i + 2], rgba[i + 3]])
        };
        let blend = |dst: &mut [u8; 4], src: [u8; 4]| {
            let a = u32::from(src[3]);
            for c in 0..3 {
                dst[c] =
                    ((u32::from(src[c]) * a + u32::from(dst[c]) * (255 - a) + 127) / 255) as u8;
            }
        };
        let mut out = Vec::with_capacity((ww * wh * 4) as usize);
        for y in 0..wh {
            for x in 0..ww {
                let mut pixel = sample(rect, &game, x, y).unwrap_or([0, 0, 0, 0xff]);
                if backdrop {
                    blend(&mut pixel, DIM);
                }
                if let Some((r, rgba)) = menu.as_ref()
                    && let Some(src) = sample(*r, rgba, x, y)
                    && src != CLEAR
                {
                    blend(&mut pixel, src);
                }
                out.extend_from_slice(&pixel);
            }
        }
        (ww, wh, out)
    }

    let shot = |label: &str, name: &str, (w, h, rgba): &(u32, u32, Vec<u8>)| {
        save_png(&out.join(format!("{label}-{name}.png")), *w, *h, rgba);
        save_png(
            &out.join(format!("{label}-{name}-2x.png")),
            w * 2,
            h * 2,
            &scaled(*w, *h, rgba, 2),
        );
    };
    let closeup = |label: &str, name: &str, (w, h, rgba): &(u32, u32, Vec<u8>), area| {
        let (cw, ch, part) = crop(*w, *h, rgba, area);
        save_png(
            &out.join(format!("{label}-closeup-{name}-4x.png")),
            cw * 4,
            ch * 4,
            &scaled(cw, ch, &part, 4),
        );
    };

    for case in &cases {
        let game = unique_scratch_dir("title-layer-review");
        for entry in fs::read_dir(&profile).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_file() {
                copy_asset_writable(&entry.path(), &game.join(entry.file_name())).unwrap();
            }
        }
        let cli: Vec<CliSetting> = if case.grow {
            vec![CliSetting::Enable("display.grow_left".into())]
        } else {
            Vec::new()
        };
        let mut enhancements = resolve(ResolveInput {
            stored: None,
            cli: &cli,
            lock: LockState::Off,
            caps: HostCapabilities::ALL,
        });
        enhancements.toggles.set(Toggle::MenuTitleLayer, true);
        let mut launch = VisualLaunchConfig {
            enhancements,
            ..VisualLaunchConfig::classic()
        };
        if case.slots {
            launch
                .enhancements
                .toggles
                .set(Toggle::MenuNamedSlots, true);
            let image = fs::read(game.join(SAVED_GAM_FILENAME)).unwrap();
            let mut older = info("s1", "Morning walk", 400);
            older.clock = [139, 4, 2, 7, 30];
            launch.menu_host = Some(Slots::new(
                &game,
                vec![
                    (older, image.clone()),
                    (info("s2", "Second try", 500), image),
                ],
            ));
        }
        let intro = crate::new_visual_intro_state(
            game.clone(),
            u5_runtime::TileGraphicsDepth::Ega16,
            Arc::new(Mutex::new(None)),
            None,
        );
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default(), InputPlugin))
            .init_asset::<Image>()
            .insert_resource(TimeUpdateStrategy::ManualDuration(FRAME));
        crate::build_visual_intro_app_with_speaker(
            &mut app,
            intro,
            crate::ScreenshotConfig {
                path: None,
                frame_delay: 0,
                preset_keys: Vec::new(),
            },
            &launch,
            crate::SpeakerOutput::Silent,
        );
        app.world_mut().spawn(Window {
            resolution: WindowResolution::new(case.window.0 as f32, case.window.1 as f32),
            ..default()
        });
        let mut title = Title {
            app,
            dir: game,
            next: Vec::new(),
        };
        // Skip the flourish with a key, then wait for the title menu's poll.
        title.frames(5).tap(KeyCode::Space);
        let mut waited = 0;
        while !crate::title_layer_can_open(title.intro()) {
            title.frame();
            waited += 1;
            assert!(waited < 5_000, "the title menu never polled");
        }
        title.frames(30);
        let label = case.label;
        shot(label, "before", &composed(title.app.world_mut()));
        save_png(
            &out.join(format!("{label}-window-before.png")),
            case.window.0,
            case.window.1,
            &window_shot(title.app.world_mut()).2,
        );

        // The open sweep, one frame at a time.
        title.next.push((KeyCode::F12, ButtonState::Pressed));
        let mut n = 0;
        loop {
            title.frame();
            if n == 0 {
                title.next.push((KeyCode::F12, ButtonState::Released));
            }
            shot(
                label,
                &format!("open-{n:02}"),
                &composed(title.app.world_mut()),
            );
            n += 1;
            if title.world().resource::<MenuView>().sweep.step.is_none() || n > 20 {
                break;
            }
        }
        title.frame();
        assert!(title.open());
        let layer = composed(title.app.world_mut());
        shot(label, "layer", &layer);
        let win = window_shot(title.app.world_mut());
        save_png(
            &out.join(format!("{label}-window-layer.png")),
            win.0,
            win.1,
            &win.2,
        );
        let panel = title.world().resource::<MenuView>().drawn().unwrap().panel;
        let (x0, y0, x1, y1) = panel.pixels();
        let (x0, y0, x1, y1) = (i64::from(x0), i64::from(y0), i64::from(x1), i64::from(y1));
        let mid = (x0 + x1) / 2;
        for (name, area) in [
            ("corner-top-left", (x0, y0 - 8, 40, 32)),
            ("corner-top-right", (x1 - 39, y0 - 8, 40, 32)),
            ("corner-bottom-left", (x0, y1 - 31, 40, 32)),
            ("corner-bottom-right", (x1 - 39, y1 - 31, 40, 32)),
            ("title-tab", (mid - 48, y0 - 4, 96, 20)),
            ("footer-tab", (mid - 56, y1 - 15, 112, 16)),
            ("bar-left", (x0, y0 + 8, 112, 24)),
            ("bar-right", (x1 - 111, y0 + 8, 112, 24)),
        ] {
            closeup(label, name, &layer, area);
        }
        let rows = if case.slots { 4 } else { 3 };
        for row in 1..rows {
            title.tap(KeyCode::ArrowDown);
            shot(
                label,
                &format!("row-{row}"),
                &composed(title.app.world_mut()),
            );
            closeup(
                label,
                &format!("row-{row}-bar"),
                &composed(title.app.world_mut()),
                (x0 + 40, y0 + 8, 240, 48),
            );
        }
        // Settings from the title, then back.
        let settings_row = if case.slots { 2 } else { 1 };
        for _ in settings_row..rows - 1 {
            title.tap(KeyCode::ArrowUp);
        }
        title.tap(KeyCode::Enter);
        shot(label, "settings", &composed(title.app.world_mut()));
        title.tap(KeyCode::Escape);
        if case.slots {
            title.tap(KeyCode::ArrowUp).tap(KeyCode::Enter);
            shot(label, "load", &composed(title.app.world_mut()));
            title.tap(KeyCode::Escape);
        }
        // The close sweep.
        title.next.push((KeyCode::Escape, ButtonState::Pressed));
        let mut n = 0;
        loop {
            title.frame();
            if n == 0 {
                title.next.push((KeyCode::Escape, ButtonState::Released));
            }
            shot(
                label,
                &format!("close-{n:02}"),
                &composed(title.app.world_mut()),
            );
            n += 1;
            if !title.world().resource::<MenuView>().sweep.visible() || n > 20 {
                break;
            }
        }
        assert!(!title.open());
        eprintln!("{label}: written");
    }
}

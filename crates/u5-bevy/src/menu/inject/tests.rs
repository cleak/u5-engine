//! The slot save's handshake in the real `--visual` app, headless (design
//! 3.3.4, 3.3.5; plan M4-10): a menu save is a typed save, `Y` never waits
//! in the queue with `Q`, a physical key during the save reaches nothing,
//! the refusals, and the thumbnail.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use bevy::input::ButtonState;
use u5_runtime::PlayState;
use u5_runtime::enhancements::Toggle;
use u5_runtime::test_fixtures::{
    open_grid, saved_game_seed_bytes, test_state, unique_scratch_dir, write_empty_ool_mirrors,
};
use u5_runtime::z_stats::{YesNoPromptKind, YesNoPromptSession};

use super::*;
use crate::enh::VisualLaunchConfig;
use crate::enh::equivalence::{FrameRecord, Owner, Scene, Shell, Timeline};
use crate::menu::host::NoStore;
use crate::menu::thumb::{THUMB_HEIGHT, THUMB_WIDTH, box_filter};
use crate::{TypedKey, drive_visual};

/// A scratch game folder with a save template and the two mirrors the
/// Q-save stages through. Removed on drop.
struct SaveDir(PathBuf);

impl SaveDir {
    fn new(label: &str) -> Self {
        let dir = Self::empty(label);
        fs::write(
            dir.0.join(SAVED_GAM_FILENAME),
            saved_game_seed_bytes(0x11, 0, 5, 5),
        )
        .unwrap();
        write_empty_ool_mirrors(&dir.0);
        dir
    }

    fn empty(label: &str) -> Self {
        let dir = unique_scratch_dir(&format!("slot-save-{label}"));
        fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    /// Every file in the folder, by name, with its bytes.
    fn files(&self) -> Vec<(String, Vec<u8>)> {
        let mut files: Vec<_> = fs::read_dir(&self.0)
            .unwrap()
            .flatten()
            .filter(|entry| entry.file_type().unwrap().is_file())
            .map(|entry| {
                (
                    entry.file_name().to_string_lossy().into_owned(),
                    fs::read(entry.path()).unwrap(),
                )
            })
            .collect();
        files.sort();
        files
    }
}

impl Drop for SaveDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn town() -> PlayState {
    test_state(open_grid(), 5, 5)
}

fn scene(dir: &Path) -> Scene {
    Scene {
        game_dir: dir.to_path_buf(),
        ..Scene::synthetic()
    }
}

/// The menu with named slots and a store: the handshake is in the app.
fn slots_launch() -> VisualLaunchConfig {
    let mut launch = VisualLaunchConfig::classic();
    launch.enhancements.toggles.set(Toggle::MenuOverlay, true);
    launch
        .enhancements
        .toggles
        .set(Toggle::MenuNamedSlots, true);
    launch.menu_host = Some(Arc::new(NoStore));
    launch
}

/// What one frame of a run showed beyond its [`FrameRecord`].
#[derive(Clone, Debug, Default)]
struct Seen {
    /// The keys queued for the game just before the input driver read.
    queue_at_read: Vec<TypedKey>,
    /// Keys the synthetic-key owner still held back after the frame.
    owner_queued: usize,
    /// The save prompt was open after the frame.
    prompt: bool,
    finished: Vec<SlotSaveFinished>,
    /// The game's frame as the handshake saw it in the frame it finished.
    frame_at_finish: Option<Vec<u8>>,
}

/// Recorded by the test systems below, one entry per frame.
#[derive(Resource, Default)]
struct Watch {
    queue_at_read: Vec<Vec<TypedKey>>,
    frame_at_finish: Vec<Option<Vec<u8>>>,
}

fn watch_queue(visual: Option<Res<VisualState>>, mut watch: ResMut<Watch>) {
    let queue = visual.map_or_else(Vec::new, |visual| {
        visual.typed_keys.iter().copied().collect()
    });
    watch.queue_at_read.push(queue);
}

fn watch_finish(
    mut finished: EventReader<SlotSaveFinished>,
    visual: Option<Res<VisualState>>,
    images: Res<Assets<Image>>,
    mut watch: ResMut<Watch>,
) {
    let frame = finished
        .read()
        .any(|event| matches!(event, SlotSaveFinished::Saved(_)))
        .then(|| {
            visual
                .and_then(|visual| images.get(&visual.image_handle))
                .and_then(|image| image.data.clone())
        })
        .flatten();
    watch.frame_at_finish.push(frame);
}

/// The test's watchers, around the input driver and the handshake.
fn install_watch(app: &mut App) {
    app.init_resource::<Watch>().add_systems(
        Update,
        (
            watch_queue.after(EnhSet::Feed).before(drive_visual),
            watch_finish
                .in_set(EnhSet::Observe)
                .after(advance_slot_save),
        ),
    );
}

/// A headless run of `timeline`, sending [`SlotSaveRequest`] before frame
/// `request_at` when it is `Some`.
struct Run {
    shell: Shell,
    records: Vec<FrameRecord>,
    seen: Vec<Seen>,
}

impl Run {
    fn new(state: PlayState, dir: &Path, launch: &VisualLaunchConfig) -> Self {
        let watch = launch.menu_host.is_some();
        let shell = Shell::new(state, &scene(dir), launch, |app| {
            if watch {
                install_watch(app);
            }
        });
        Self {
            shell,
            records: Vec::new(),
            seen: Vec::new(),
        }
    }

    fn play(&mut self, timeline: &Timeline, request_at: Option<usize>, hold: bool) {
        for index in 0..timeline.len() {
            if request_at == Some(index) {
                self.shell.app.world_mut().send_event(SlotSaveRequest {
                    hold_for_store: hold,
                });
            }
            let record = self.shell.step(timeline.frame(index));
            self.records.push(record);
            let seen = self.observe();
            self.seen.push(seen);
        }
    }

    fn observe(&mut self) -> Seen {
        let world = self.shell.app.world_mut();
        let mut seen = Seen {
            prompt: save_prompt_open(world.resource::<VisualState>()),
            ..Seen::default()
        };
        if let Some(owner) = world.get_resource::<SyntheticInputOwner>() {
            seen.owner_queued = owner.queued();
        }
        if let Some(events) = world.get_resource::<Events<SlotSaveFinished>>() {
            seen.finished = events.iter_current_update_events().cloned().collect();
        }
        if let Some(mut watch) = world.get_resource_mut::<Watch>() {
            seen.queue_at_read = watch.queue_at_read.pop().unwrap_or_default();
            seen.frame_at_finish = watch.frame_at_finish.pop().flatten();
            watch.queue_at_read.clear();
            watch.frame_at_finish.clear();
        }
        seen
    }

    fn visual(&self) -> &VisualState {
        self.shell.visual()
    }

    fn world(&self) -> &World {
        self.shell.app.world()
    }

    fn handshake(&self) -> &SlotSaveHandshake {
        self.world().resource::<SlotSaveHandshake>()
    }

    fn finished(&self) -> Vec<(usize, SlotSaveFinished)> {
        self.seen
            .iter()
            .enumerate()
            .flat_map(|(frame, seen)| {
                seen.finished
                    .iter()
                    .map(move |event| (frame, event.clone()))
            })
            .collect()
    }

    /// The frames whose read found these keys queued by the owner, in order.
    fn fed(&self) -> Vec<(usize, KeyCode)> {
        self.seen
            .iter()
            .enumerate()
            .flat_map(|(frame, seen)| seen.queue_at_read.iter().map(move |key| (frame, key.key)))
            .collect()
    }
}

/// Idle frames, the menu opened with F12 at frame 10 (the feature's own
/// key), and time for the save.
const OPEN_AT: usize = 10;
const REQUEST_AT: usize = 20;
const FRAMES: usize = 140;

fn menu_timeline() -> Timeline {
    let mut timeline = Timeline::new(FRAMES);
    timeline.feature_tap(OPEN_AT, KeyCode::F12);
    timeline
}

/// The menu's save, run to its end.
fn menu_save(dir: &SaveDir, timeline: &Timeline) -> Run {
    let mut run = Run::new(town(), &dir.0, &slots_launch());
    run.play(timeline, Some(REQUEST_AT), false);
    run
}

fn saved(run: &Run) -> SlotSaveCapture {
    let finished = run.finished();
    match finished.as_slice() {
        [(_, SlotSaveFinished::Saved(capture))] => capture.clone(),
        other => panic!("one save expected, got {other:?}"),
    }
}

#[test]
fn the_menu_opens_and_save_is_offered_at_the_prompt() {
    let dir = SaveDir::new("offered");
    let mut run = Run::new(town(), &dir.0, &slots_launch());
    run.play(&Timeline::new(OPEN_AT + 3), None, false);
    let mut timeline = Timeline::new(OPEN_AT + 3);
    timeline.feature_tap(1, KeyCode::F12);
    run.play(&timeline, None, false);
    assert_eq!(
        run.world().resource::<OverlayState>().open(),
        Some(OverlayId::Menu)
    );
    let snap = crate::menu::world_snapshot(run.world());
    assert!(snap.slots);
    assert!(snap.save_available, "Q opens the save prompt here");
    // Another owner's token greys it.
    run.shell
        .app
        .world_mut()
        .resource_mut::<SyntheticInputOwner>()
        .take(SyntheticOwner::StartKeys)
        .unwrap();
    assert!(!crate::menu::world_snapshot(run.world()).save_available);
}

#[test]
fn a_menu_save_is_byte_equal_to_a_typed_save() {
    let menu_dir = SaveDir::new("menu");
    let typed_dir = SaveDir::new("typed");
    let timeline = menu_timeline();
    let menu = menu_save(&menu_dir, &timeline);
    let capture = saved(&menu);
    assert!(
        capture.summary.avatar.is_some(),
        "the save holds the Avatar"
    );

    // The player types the same two keys in the frames the menu fed them.
    let fed = menu.fed();
    assert_eq!(
        fed.iter().map(|(_, key)| *key).collect::<Vec<_>>(),
        [KeyCode::KeyQ, KeyCode::KeyY],
        "{fed:?}"
    );
    let mut typed_timeline = timeline.without_feature();
    for (frame, key) in &fed {
        typed_timeline.game_tap(*frame, *key);
    }
    let mut typed = Run::new(town(), &typed_dir.0, &VisualLaunchConfig::classic());
    typed.play(&typed_timeline, None, false);

    assert_eq!(menu.records.len(), typed.records.len());
    for (frame, (a, b)) in menu.records.iter().zip(&typed.records).enumerate() {
        assert_eq!(
            a.digest, b.digest,
            "frame {frame}: state, PRNG, turn, transcript"
        );
        assert_eq!(a.frame, b.frame, "frame {frame}: the game's picture");
    }
    assert_eq!(
        menu.visual().state.message_entries(),
        typed.visual().state.message_entries()
    );
    let (menu_files, typed_files) = (menu_dir.files(), typed_dir.files());
    assert_eq!(
        menu_files.iter().map(|(name, _)| name).collect::<Vec<_>>(),
        typed_files.iter().map(|(name, _)| name).collect::<Vec<_>>()
    );
    for ((name, a), (_, b)) in menu_files.iter().zip(&typed_files) {
        assert!(a == b, "{name} differs");
    }
    let saved_gam = &menu_files
        .iter()
        .find(|(name, _)| name == SAVED_GAM_FILENAME)
        .unwrap()
        .1;
    assert_eq!(
        &capture.image, saved_gam,
        "the capture is the file as written"
    );
    assert_eq!(capture.turn, menu.visual().state.turn);
    assert!(!menu.handshake().busy());
    assert_eq!(
        menu.world().resource::<SyntheticInputOwner>().holder(),
        None,
        "the token is released"
    );
}

#[test]
fn y_is_never_queued_with_q_under_buffer_off() {
    let dir = SaveDir::new("buffer-off");
    let state = town();
    assert!(!state.typeahead_buffer_enabled, "Buffer Off");
    let mut run = Run::new(state, &dir.0, &slots_launch());
    run.play(&menu_timeline(), Some(REQUEST_AT), false);
    let _ = saved(&run);
    let mut prompt_seen_at = None;
    for (frame, seen) in run.seen.iter().enumerate() {
        let keys: Vec<KeyCode> = seen.queue_at_read.iter().map(|typed| typed.key).collect();
        assert!(
            !(keys.contains(&KeyCode::KeyQ) && keys.contains(&KeyCode::KeyY)),
            "frame {frame}: {keys:?}"
        );
        assert!(seen.owner_queued <= 1, "frame {frame}: one key at a time");
        if keys.contains(&KeyCode::KeyY) {
            let shown = prompt_seen_at.expect("Y only once the prompt showed");
            assert!(frame > shown, "Y on a later frame than the prompt's");
        }
        if seen.prompt && prompt_seen_at.is_none() {
            prompt_seen_at = Some(frame);
        }
    }
    assert!(prompt_seen_at.is_some());
}

#[test]
fn a_physical_key_during_the_handshake_reaches_nothing() {
    // The save itself takes two frames here (Q, then Y and the write), so
    // the slot copy's hold keeps the token for the rest: the player types
    // through both.
    let quiet_dir = SaveDir::new("quiet");
    let noisy_dir = SaveDir::new("noisy");
    let mut quiet = Run::new(town(), &quiet_dir.0, &slots_launch());
    quiet.play(&menu_timeline(), Some(REQUEST_AT), true);
    let start = quiet.fed()[0].0;
    // A step, a yes, a no, an Escape, F12 and a pass, from the frame after
    // the menu's Q to well into the copy.
    let mut noisy_timeline = menu_timeline();
    let keys = [
        KeyCode::ArrowUp,
        KeyCode::KeyY,
        KeyCode::KeyN,
        KeyCode::Escape,
        KeyCode::F12,
        KeyCode::Space,
    ];
    for (i, frame) in (start + 1..start + 60).step_by(2).enumerate() {
        noisy_timeline.game_tap(frame, keys[i % keys.len()]);
    }
    let mut noisy = Run::new(town(), &noisy_dir.0, &slots_launch());
    noisy.play(&noisy_timeline, Some(REQUEST_AT), true);
    let _ = saved(&noisy);
    assert_eq!(noisy.handshake().phase(), SlotSavePhase::Storing);
    assert_eq!(quiet.records, noisy.records, "no key changed the game");
    assert_eq!(quiet.fed(), noisy.fed(), "and none reached its queue");
    assert_eq!(
        noisy.world().resource::<OverlayState>().open(),
        None,
        "F12 opened no menu"
    );
    assert_eq!(quiet_dir.files(), noisy_dir.files());
}

#[test]
fn the_token_is_kept_for_the_store_and_released_after() {
    let dir = SaveDir::new("hold");
    let mut run = Run::new(town(), &dir.0, &slots_launch());
    run.play(&menu_timeline(), Some(REQUEST_AT), true);
    let _ = saved(&run);
    assert_eq!(run.handshake().phase(), SlotSavePhase::Storing);
    assert_eq!(
        run.world().resource::<SyntheticInputOwner>().holder(),
        Some(SyntheticOwner::MenuSlotSave)
    );
    finish_store(&mut run);
    let world = run.world();
    assert_eq!(world.resource::<SyntheticInputOwner>().holder(), None);
    assert!(!world.resource::<SlotSaveHandshake>().busy());
}

#[test]
fn away_from_the_prompt_nothing_is_typed() {
    let dir = SaveDir::new("refused");
    let mut state = town();
    state.active_yes_no_prompt = Some(YesNoPromptSession {
        kind: YesNoPromptKind::ExitToDos,
    });
    let mut run = Run::new(state, &dir.0, &slots_launch());
    run.play(&Timeline::new(40), Some(REQUEST_AT), false);
    assert_eq!(run.finished(), [(REQUEST_AT, SlotSaveFinished::Refused)]);
    assert!(run.fed().is_empty());
    assert_eq!(run.world().resource::<SyntheticInputOwner>().holder(), None);

    // A drunk party: Q might stagger, so the menu does not type it.
    let mut state = town();
    state.town_drunkenness_counter = 2;
    let mut run = Run::new(state, &dir.0, &slots_launch());
    run.play(&Timeline::new(40), Some(REQUEST_AT), false);
    assert_eq!(run.finished(), [(REQUEST_AT, SlotSaveFinished::Refused)]);
    assert!(run.fed().is_empty());
}

/// Opens a direction prompt in the frame the menu's `Q` is fed, so `Q` goes
/// to that prompt instead.
fn steal_the_q(owner: Res<SyntheticInputOwner>, mut visual: ResMut<VisualState>) {
    if owner.holder() == Some(SyntheticOwner::MenuSlotSave)
        && visual
            .typed_keys
            .iter()
            .any(|typed| typed.key == KeyCode::KeyQ)
        && visual.state.active_yes_no_prompt.is_none()
    {
        visual.state.active_yes_no_prompt = Some(YesNoPromptSession {
            kind: YesNoPromptKind::ExitToDos,
        });
    }
}

#[test]
fn another_prompt_stops_the_save_and_types_no_y() {
    let dir = SaveDir::new("unavailable");
    let mut run = Run::new(town(), &dir.0, &slots_launch());
    run.shell
        .app
        .add_systems(Update, steal_the_q.after(EnhSet::Feed).before(drive_visual));
    run.play(&menu_timeline(), Some(REQUEST_AT), false);
    let finished = run.finished();
    assert!(
        matches!(finished.as_slice(), [(_, SlotSaveFinished::Unavailable)]),
        "{finished:?}"
    );
    let fed: Vec<KeyCode> = run.fed().into_iter().map(|(_, key)| key).collect();
    assert_eq!(fed, [KeyCode::KeyQ], "no Y");
    assert_eq!(run.world().resource::<SyntheticInputOwner>().holder(), None);
    let toasts = run.world().resource::<Toasts>();
    assert_eq!(toasts.current(), Some(t::SAVE_UNAVAILABLE));
}

#[test]
fn a_save_that_cannot_be_written_fails_with_its_toast() {
    // No template: the game's own save reports its error.
    let dir = SaveDir::empty("no-template");
    let mut run = Run::new(town(), &dir.0, &slots_launch());
    run.play(&menu_timeline(), Some(REQUEST_AT), false);
    let finished = run.finished();
    assert!(
        matches!(finished.as_slice(), [(_, SlotSaveFinished::Failed(_))]),
        "{finished:?}"
    );
    assert_eq!(
        run.world().resource::<Toasts>().current(),
        Some(t::SAVE_FAILED)
    );
    assert_eq!(run.world().resource::<SyntheticInputOwner>().holder(), None);
}

#[test]
fn an_old_save_does_not_pass_the_check() {
    let dir = SaveDir::new("old");
    let later = SystemTime::now() + std::time::Duration::from_secs(60);
    let checked = check_working_save(&dir.0, later);
    assert_eq!(
        checked.map(|_| ()),
        Err(NO_AVATAR.to_string()),
        "no Avatar first"
    );
    let mut image = saved_game_seed_bytes(0x11, 0, 5, 5);
    image[u5_runtime::SAVE_AVATAR_NAME_OFFSET] = b'A';
    fs::write(dir.0.join(SAVED_GAM_FILENAME), &image).unwrap();
    assert_eq!(
        check_working_save(&dir.0, later).map(|_| ()),
        Err(NOT_WRITTEN.to_string())
    );
    let (bytes, summary) = check_working_save(&dir.0, whole_second(SystemTime::now())).unwrap();
    assert_eq!(bytes, image);
    assert_eq!(summary.avatar.as_deref(), Some("A"));
}

#[test]
fn the_thumbnail_is_the_box_filtered_game_frame() {
    let dir = SaveDir::new("thumb");
    let run = menu_save(&dir, &menu_timeline());
    let capture = saved(&run);
    let frame = run
        .seen
        .iter()
        .find_map(|seen| seen.frame_at_finish.clone())
        .expect("the frame at the save");
    assert_eq!(
        frame.len(),
        (320 * 200 * 4) as usize,
        "the faithful 320x200 frame"
    );
    let png = capture.thumbnail_png.expect("a thumbnail");
    let decoded = image::load_from_memory_with_format(&png, image::ImageFormat::Png)
        .unwrap()
        .to_rgba8();
    assert_eq!(decoded.dimensions(), (THUMB_WIDTH, THUMB_HEIGHT));
    assert_eq!(decoded.into_raw(), box_filter(&frame).unwrap());
}

#[test]
fn the_saving_toast_shows_while_the_game_saves() {
    let dir = SaveDir::new("toast");
    let mut run = Run::new(town(), &dir.0, &slots_launch());
    run.play(&menu_timeline(), Some(REQUEST_AT), true);
    let _ = saved(&run);
    assert_eq!(run.world().resource::<Toasts>().current(), Some(t::SAVING));
}

/// Ends the slot copy the request held the token for.
fn finish_store(run: &mut Run) {
    run.shell
        .app
        .world_mut()
        .resource_scope(|world, mut handshake: Mut<SlotSaveHandshake>| {
            handshake.finish_store(&mut world.resource_mut::<SyntheticInputOwner>());
        });
}

#[test]
fn held_keys_stay_latched_across_the_save() {
    // A key pressed during the save and still held when it lets the
    // keyboard go is not a press: it is ignored until released.
    let run = |held: bool| {
        let dir = SaveDir::new(if held { "latch-held" } else { "latch-quiet" });
        let mut run = Run::new(town(), &dir.0, &slots_launch());
        let mut during = Timeline::new(60);
        during.feature_tap(OPEN_AT, KeyCode::F12);
        if held {
            during.event(
                REQUEST_AT + 10,
                KeyCode::ArrowLeft,
                ButtonState::Pressed,
                false,
                Owner::Game,
            );
        }
        run.play(&during, Some(REQUEST_AT), true);
        assert_eq!(run.handshake().phase(), SlotSavePhase::Storing);
        finish_store(&mut run);
        let mut after = Timeline::new(60);
        if held {
            for frame in (3..30).step_by(3) {
                after.event(
                    frame,
                    KeyCode::ArrowLeft,
                    ButtonState::Pressed,
                    true,
                    Owner::Game,
                );
            }
            after.event(
                30,
                KeyCode::ArrowLeft,
                ButtonState::Released,
                false,
                Owner::Game,
            );
        }
        run.play(&after, None, false);
        (run.records, dir.files())
    };
    let quiet = run(false);
    let held = run(true);
    let first = quiet.0.iter().zip(&held.0).position(|(a, b)| a != b);
    assert_eq!(first, None, "the held key never moved the party");
    assert_eq!(quiet.1, held.1);
}

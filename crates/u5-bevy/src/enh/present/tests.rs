//! The presentation pipeline host with a test-only step: zero footprint,
//! the faithful image left alone, revision repaints, the recompose
//! triggers, step order and the presented image's lifetime (design 1.3.8,
//! plan M0-15).

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use u5_runtime::enhancements::{LockReason, ResolvedEnhancements, TOGGLES, Toggle};

use super::*;
use crate::enh::overlay::tests::idle_visual_state;
use crate::enh::{VisualLaunchConfig, add_enhancements, configure_enh_sets};
use crate::{VISUAL_PLAY_FRAME_HEIGHT, VISUAL_PLAY_FRAME_WIDTH};

const FRAME: UVec2 = UVec2::new(VISUAL_PLAY_FRAME_WIDTH, VISUAL_PLAY_FRAME_HEIGHT);

/// What a test step saw: its order and the faithful hash, per apply.
type ApplyLog = Arc<Mutex<Vec<(StepOrder, u64)>>>;

/// The test-only step. It XORs one byte into every pixel's red channel of
/// the copy, so the presented image always differs from the faithful one,
/// and reports a revision the test sets.
struct TestStep {
    order: StepOrder,
    xor: u8,
    revision: Arc<AtomicU64>,
    applied: ApplyLog,
    /// A size to give the presented copy, for the lifetime test.
    resize: Option<Arc<Mutex<Option<UVec2>>>>,
}

impl TestStep {
    fn new(order: StepOrder) -> Self {
        Self {
            order,
            xor: 0x5a,
            revision: Arc::default(),
            applied: Arc::default(),
            resize: None,
        }
    }
}

impl PresentStep for TestStep {
    fn order(&self) -> StepOrder {
        self.order
    }

    fn revision(&mut self, _input: &PresentInput) -> u64 {
        self.revision.load(Ordering::SeqCst)
    }

    fn apply(&mut self, input: &PresentInput, frame: &mut PresentedFrame) {
        self.applied
            .lock()
            .unwrap()
            .push((self.order, input.faithful_hash));
        if let Some(size) = self.resize.as_ref().and_then(|size| *size.lock().unwrap()) {
            frame.resize(size);
            return;
        }
        for pixel in frame.rgba.as_chunks_mut::<4>().0 {
            pixel[0] ^= self.xor;
        }
    }
}

/// The test step's picture of `faithful`.
fn stepped(faithful: &[u8], xor: u8) -> Vec<u8> {
    let mut rgba = faithful.to_vec();
    for pixel in rgba.as_chunks_mut::<4>().0 {
        pixel[0] ^= xor;
    }
    rgba
}

fn has_system(app: &mut App, name: &str) -> bool {
    app.world_mut()
        .resource_mut::<bevy::ecs::schedule::Schedules>()
        .iter()
        .any(|(_, schedule)| {
            schedule
                .graph()
                .systems()
                .any(|(_, system, _)| system.name().contains(name))
        })
}

/// A play frame on a real image, a window and the frame sprite, as the
/// shell has them; the foundation's sets, and nothing of the pipeline.
fn app() -> App {
    let mut images = Assets::<Image>::default();
    let visual = idle_visual_state(&mut images);
    let mut app = App::new();
    app.insert_resource(images).insert_resource(visual);
    configure_enh_sets(&mut app);
    let mut window = Window::default();
    window.resolution.set(1280.0, 960.0);
    app.world_mut().spawn(window);
    let image = faithful_handle(&app);
    app.world_mut().spawn((
        Sprite {
            image,
            custom_size: Some(Vec2::new(960.0, 720.0)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));
    app
}

fn faithful_handle(app: &App) -> Handle<Image> {
    app.world().resource::<VisualState>().image_handle.clone()
}

fn sprite_image(app: &mut App) -> Handle<Image> {
    let mut query = app.world_mut().query::<&Sprite>();
    query.single(app.world()).unwrap().image.clone()
}

fn image_bytes(app: &App, handle: &Handle<Image>) -> Vec<u8> {
    app.world()
        .resource::<Assets<Image>>()
        .get(handle)
        .and_then(|image| image.data.clone())
        .expect("image bytes")
}

/// Writes the faithful image the way the upload sites do.
fn upload(app: &mut App, rgba: Vec<u8>) {
    let handle = faithful_handle(app);
    app.world_mut()
        .resource_mut::<Assets<Image>>()
        .get_mut(&handle)
        .unwrap()
        .data = Some(rgba);
}

/// A distinct frame for each `seed`.
fn pattern(seed: u8) -> Vec<u8> {
    (0..FRAME.x * FRAME.y)
        .flat_map(|i| {
            let v = (i as u8).wrapping_mul(seed | 1).wrapping_add(seed);
            [v, v.rotate_left(3), seed, 0xff]
        })
        .collect()
}

fn composes(app: &App) -> u64 {
    app.world().resource::<PresentHost>().composes()
}

fn presented(app: &App) -> Option<Handle<Image>> {
    app.world().resource::<PresentHost>().presented().cloned()
}

// ---------------------------------------------------------------------------
// Zero footprint.

#[test]
fn no_host_exists_while_no_step_is_registered() {
    let mut every = VisualLaunchConfig::classic();
    // ...except the grow-left layout, whose `[grow-left compose]` step,
    // frame fit and canvas size are its own (M1-09, display/grow tests).
    for spec in TOGGLES
        .iter()
        .filter(|spec| spec.toggle != Toggle::DisplayGrowLeft)
    {
        every.enhancements.toggles.set(spec.toggle, true);
    }
    for launch in [
        VisualLaunchConfig::classic(),
        VisualLaunchConfig {
            enhancements: ResolvedEnhancements::locked(LockReason::CliFlag),
            ..VisualLaunchConfig::classic()
        },
        every,
    ] {
        let mut app = app();
        // With every toggle on, the display chords (F11, Ctrl+P) bind, so
        // the hotkey router runs and reads the keyboard.
        app.init_resource::<ButtonInput<KeyCode>>();
        add_enhancements(&mut app, &launch);
        assert!(!app.world().contains_resource::<PresentSteps>());
        assert!(!app.world().contains_resource::<PresentHost>());
        assert!(!has_system(&mut app, "present_frame"));
        let faithful = faithful_handle(&app);
        for seed in 0..3 {
            upload(&mut app, pattern(seed));
            app.update();
            assert_eq!(sprite_image(&mut app), faithful, "the faithful handle");
        }
        assert_eq!(app.world().resource::<Assets<Image>>().len(), 1);
    }
}

#[test]
fn pipeline_active_follows_the_step_list() {
    let mut app = App::new();
    let world = app.world_mut();
    let mut check = IntoSystem::into_system(pipeline_active);
    check.initialize(world);
    assert!(!check.run((), world), "no pipeline: Classic");
    world.init_resource::<PresentSteps>();
    assert!(!check.run((), world), "no step");
    world
        .resource_mut::<PresentSteps>()
        .register(TestStep::new(StepOrder::HIGHLIGHT));
    assert!(check.run((), world));
    world
        .resource_mut::<PresentSteps>()
        .remove(StepOrder::HIGHLIGHT);
    assert!(!check.run((), world));
}

#[test]
fn registering_twice_adds_the_host_once() {
    let mut app = App::new();
    register_present_step(&mut app, TestStep::new(StepOrder::HIGHLIGHT));
    register_present_step(&mut app, TestStep::new(StepOrder::PRESCALE));
    register_present_step(&mut app, TestStep::new(StepOrder::HIGHLIGHT));
    let schedules = app.world().resource::<bevy::ecs::schedule::Schedules>();
    let count = schedules
        .iter()
        .flat_map(|(_, schedule)| schedule.graph().systems())
        .filter(|(_, system, _)| system.name().contains("present_frame"))
        .count();
    assert_eq!(count, 1);
    assert_eq!(
        app.world().resource::<PresentSteps>().orders(),
        [StepOrder::HIGHLIGHT, StepOrder::PRESCALE],
        "one step per order"
    );
}

// ---------------------------------------------------------------------------
// The faithful image.

#[test]
fn the_faithful_image_is_never_written() {
    let mut off = app();
    let mut on = app();
    register_present_step(&mut on, TestStep::new(StepOrder::HIGHLIGHT));
    let faithful = faithful_handle(&on);
    for seed in 0..6 {
        let rgba = pattern(seed);
        upload(&mut off, rgba.clone());
        upload(&mut on, rgba.clone());
        off.update();
        on.update();
        let faithful_on = image_bytes(&on, &faithful);
        assert_eq!(faithful_on, image_bytes(&off, &faithful_handle(&off)));
        assert_eq!(hash_bytes(&faithful_on), hash_bytes(&rgba));
        let presented = presented(&on).expect("a presented copy");
        assert_ne!(presented, faithful);
        assert_eq!(
            sprite_image(&mut on),
            presented,
            "the sprite shows the copy"
        );
        assert_eq!(image_bytes(&on, &presented), stepped(&rgba, 0x5a));
        assert_eq!(sprite_image(&mut off), faithful_handle(&off));
    }
}

/// Feeds each `(label, hash, rgba)` play frame through an app with the test step
/// and one without, as an upload site would, and checks that the faithful
/// bytes and hash stay those of the frame and the copy is the step's.
fn assert_frames_pass_untouched(frames: impl Iterator<Item = (String, u64, Vec<u8>)>) {
    assert_frames_pass_untouched_by(
        frames,
        || TestStep::new(StepOrder::HIGHLIGHT),
        |rgba| stepped(rgba, 0x5a),
    );
}

/// [`assert_frames_pass_untouched`] with `make`'s step, whose copy of a
/// frame is `expect` of it.
fn assert_frames_pass_untouched_by<S: PresentStep>(
    frames: impl Iterator<Item = (String, u64, Vec<u8>)>,
    make: impl Fn() -> S,
    expect: impl Fn(&[u8]) -> Vec<u8>,
) {
    let mut off = app();
    let mut on = app();
    register_present_step(&mut on, make());
    let mut count = 0;
    for (label, hash, rgba) in frames {
        assert_eq!(hash_bytes(&rgba), hash, "{label}: the suite's own hash");
        assert_eq!(rgba.len(), (FRAME.x * FRAME.y * 4) as usize, "{label}");
        for app in [&mut off, &mut on] {
            upload(app, rgba.clone());
            app.update();
        }
        let faithful_on = image_bytes(&on, &faithful_handle(&on));
        assert_eq!(hash_bytes(&faithful_on), hash, "{label}: faithful hash");
        assert_eq!(
            faithful_on,
            image_bytes(&off, &faithful_handle(&off)),
            "{label}: on equals off"
        );
        let presented = presented(&on).expect("a presented copy");
        assert_eq!(
            image_bytes(&on, &presented),
            expect(&rgba),
            "{label}: the copy"
        );
        count += 1;
    }
    assert!(count > 0, "no frame was checked");
}

/// The brightness step (M2-03) at 70% over the whole copy, and what it
/// makes of a frame.
fn brightness_step() -> crate::display::light::BrightnessStep {
    use u5_runtime::enhancements::{BrightnessScope, DisplayLight};
    crate::display::light::BrightnessStep::of(&DisplayLight {
        brightness: 70,
        scope: BrightnessScope::Frame,
        ..DisplayLight::default()
    })
    .expect("a step at 70")
}

fn brightened(rgba: &[u8]) -> Vec<u8> {
    let mut out = rgba.to_vec();
    u5_runtime::enhancements::BrightnessLut::for_percent(70).apply_rgba(&mut out);
    out
}

#[test]
fn synthetic_frames_pass_untouched() {
    // Synthetic frames; the real frame and
    // route suites run in `suites_pass_untouched` with the assets.
    let frames = (0..4u8).map(|seed| {
        let rgba = pattern(seed.wrapping_mul(37));
        (format!("pattern-{seed}"), hash_bytes(&rgba), rgba)
    });
    assert_frames_pass_untouched(frames);
    let frames = (0..4u8).map(|seed| {
        let rgba = pattern(seed.wrapping_mul(37));
        (format!("pattern-{seed}"), hash_bytes(&rgba), rgba)
    });
    assert_frames_pass_untouched_by(frames, brightness_step, brightened);
}

// ---------------------------------------------------------------------------
// The triggers.

#[test]
fn a_revision_change_repaints_an_unchanged_faithful_image() {
    let mut app = app();
    let step = TestStep::new(StepOrder::HIGHLIGHT);
    let revision = step.revision.clone();
    let applied = step.applied.clone();
    register_present_step(&mut app, step);
    upload(&mut app, pattern(1));
    app.update();
    assert_eq!(composes(&app), 1);
    let faithful_before = image_bytes(&app, &faithful_handle(&app));

    revision.store(7, Ordering::SeqCst);
    app.update();
    assert_eq!(composes(&app), 2, "the new revision repaints");
    assert_eq!(applied.lock().unwrap().len(), 2);
    assert_eq!(image_bytes(&app, &faithful_handle(&app)), faithful_before);

    app.update();
    assert_eq!(composes(&app), 2, "the same revision does not");
}

#[test]
fn recompose_only_on_the_listed_triggers() {
    let mut app = app();
    register_present_step(&mut app, TestStep::new(StepOrder::HIGHLIGHT));
    upload(&mut app, pattern(1));
    app.update();
    let mut expected = 1;
    assert_eq!(composes(&app), expected);

    // Not triggers: idle frames, the same bytes uploaded again, and game
    // state the frame was not drawn from.
    for _ in 0..3 {
        app.update();
    }
    upload(&mut app, pattern(1));
    app.update();
    {
        let mut visual = app.world_mut().resource_mut::<VisualState>();
        visual.prompt_cursor_frame = visual.prompt_cursor_frame.wrapping_add(1);
        visual.input_line.push('x');
        visual.state.turn = visual.state.turn.wrapping_add(1);
    }
    app.update();
    assert_eq!(composes(&app), expected, "nothing listed changed");

    // The faithful hash.
    upload(&mut app, pattern(2));
    app.update();
    expected += 1;
    assert_eq!(composes(&app), expected, "faithful hash");

    // The animation tick.
    app.world_mut()
        .resource_mut::<VisualState>()
        .state
        .animation
        .tick_static_tiles();
    app.update();
    expected += 1;
    assert_eq!(composes(&app), expected, "animation tick");

    // A flash: the camp inversion, then the roster row.
    app.world_mut()
        .resource_mut::<VisualState>()
        .state
        .presentation_camp_flash_inverted = true;
    app.update();
    expected += 1;
    assert_eq!(composes(&app), expected, "camp flash");
    app.world_mut()
        .resource_mut::<VisualState>()
        .state
        .presentation_stats_row_flash = Some(1);
    app.update();
    expected += 1;
    assert_eq!(composes(&app), expected, "row flash");

    // The layout: the window, then the side layout.
    {
        let mut query = app.world_mut().query::<&mut Window>();
        let mut window = query.single_mut(app.world_mut()).unwrap();
        window.resolution.set(1920.0, 1080.0);
    }
    app.update();
    expected += 1;
    assert_eq!(composes(&app), expected, "window size");
    app.world_mut().insert_resource(SideLayout::default());
    app.update();
    assert_eq!(composes(&app), expected, "an empty layout moves nothing");
    app.world_mut().insert_resource(SideLayout {
        frame: Some(Rect::new(100.0, 0.0, 1540.0, 1080.0)),
        ..SideLayout::default()
    });
    app.update();
    expected += 1;
    assert_eq!(composes(&app), expected, "side layout");

    // The step list.
    app.world_mut()
        .resource_mut::<PresentSteps>()
        .register(TestStep::new(StepOrder::BRIGHTNESS));
    app.update();
    expected += 1;
    assert_eq!(composes(&app), expected, "a new step");

    for _ in 0..3 {
        app.update();
    }
    assert_eq!(composes(&app), expected, "and then quiet again");
}

// ---------------------------------------------------------------------------
// Order and lifetime.

#[test]
fn steps_run_in_their_order_whatever_the_registration_order() {
    // The design's places are strictly increasing.
    assert!(StepOrder::ALL.windows(2).all(|pair| pair[0] < pair[1]));

    let log: ApplyLog = Arc::default();
    let mut app = app();
    for (index, order) in StepOrder::ALL.into_iter().rev().enumerate() {
        let mut step = TestStep::new(order);
        step.applied = log.clone();
        step.xor = 1 << index;
        register_present_step(&mut app, step);
    }
    assert_eq!(
        app.world().resource::<PresentSteps>().orders(),
        StepOrder::ALL
    );
    upload(&mut app, pattern(3));
    app.update();
    let ran: Vec<StepOrder> = log
        .lock()
        .unwrap()
        .iter()
        .map(|(order, _)| *order)
        .collect();
    assert_eq!(ran, StepOrder::ALL);
    // Each step drew on the one before it: the five XORs all landed.
    let presented = presented(&app).unwrap();
    assert_eq!(image_bytes(&app, &presented), stepped(&pattern(3), 0x1f));
}

#[test]
fn one_presented_image_per_geometry() {
    let mut app = app();
    let size = Arc::new(Mutex::new(None));
    let mut step = TestStep::new(StepOrder::GROW_LEFT_COMPOSE);
    step.resize = Some(size.clone());
    let revision = step.revision.clone();
    register_present_step(&mut app, step);

    upload(&mut app, pattern(1));
    app.update();
    let first = presented(&app).unwrap();
    let count = app.world().resource::<Assets<Image>>().len();
    assert_eq!(count, 2, "the faithful image and one copy");

    // Same geometry: repainted in place.
    upload(&mut app, pattern(2));
    app.update();
    assert_eq!(presented(&app).unwrap(), first);
    assert_eq!(app.world().resource::<Assets<Image>>().len(), count);

    // A new size: replaced, and the old one dropped.
    *size.lock().unwrap() = Some(UVec2::new(688, 240));
    revision.store(1, Ordering::SeqCst);
    app.update();
    let wide = presented(&app).unwrap();
    assert_ne!(wide, first);
    assert_eq!(sprite_image(&mut app), wide);
    assert_eq!(app.world().resource::<Assets<Image>>().len(), count);
    let images = app.world().resource::<Assets<Image>>();
    assert!(images.get(&first).is_none());
    assert_eq!(images.get(&wide).unwrap().size(), UVec2::new(688, 240));
    assert_eq!(
        images.get(&faithful_handle(&app)).unwrap().size(),
        FRAME,
        "the faithful image keeps its size"
    );
}

#[test]
fn removing_the_last_step_shows_the_faithful_image_again() {
    let mut app = app();
    register_present_step(&mut app, TestStep::new(StepOrder::HIGHLIGHT));
    upload(&mut app, pattern(1));
    app.update();
    let copy = presented(&app).unwrap();
    assert_eq!(sprite_image(&mut app), copy);

    app.world_mut()
        .resource_mut::<PresentSteps>()
        .remove(StepOrder::HIGHLIGHT);
    app.update();
    let faithful = faithful_handle(&app);
    assert_eq!(sprite_image(&mut app), faithful);
    assert!(presented(&app).is_none());
    assert!(app.world().resource::<Assets<Image>>().get(&copy).is_none());
    assert_eq!(image_bytes(&app, &faithful), pattern(1));

    // Registered again, it presents again.
    app.world_mut()
        .resource_mut::<PresentSteps>()
        .register(TestStep::new(StepOrder::HIGHLIGHT));
    app.update();
    assert_eq!(sprite_image(&mut app), presented(&app).unwrap());
}

#[test]
fn a_respawned_frame_sprite_is_given_the_copy() {
    let mut app = app();
    register_present_step(&mut app, TestStep::new(StepOrder::HIGHLIGHT));
    upload(&mut app, pattern(1));
    app.update();
    // A sprite that shows the faithful handle again (as the intro's handle
    // reaches play) is pointed at the copy without a recompose.
    {
        let faithful = faithful_handle(&app);
        let mut query = app.world_mut().query::<&mut Sprite>();
        query.single_mut(app.world_mut()).unwrap().image = faithful;
    }
    let before = composes(&app);
    app.update();
    assert_eq!(composes(&app), before);
    assert_eq!(sprite_image(&mut app), presented(&app).unwrap());
}

// ---------------------------------------------------------------------------
// The real suites.

/// Every frame-suite and route-suite frame, through the host with the test
/// step on and off: the faithful bytes and hash are the suite's own.
#[test]
#[cfg(feature = "original-asset-tests")]
fn suites_pass_untouched() {
    use std::path::{Path, PathBuf};

    use u5_runtime::TileGraphicsDepth;

    let Some(source) = u5_runtime::test_fixtures::configured_original_asset_dir() else {
        return;
    };
    // A fresh temporary folder from the runtime's fixtures; everything
    // below goes in sub-folders of it.
    let root = u5_runtime::test_fixtures::debug_game_dir();
    let scratch = |name: &str| -> PathBuf { root.join(name) };
    // The route suite enters towns, which writes the `.OOL` mirror: a
    // writable copy, never the asset install.
    let game_dir = scratch("assets");
    std::fs::create_dir_all(&game_dir).unwrap();
    for entry in std::fs::read_dir(&source).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            u5_runtime::test_fixtures::copy_asset_writable(
                &entry.path(),
                &game_dir.join(entry.file_name()),
            )
            .unwrap();
        }
    }
    let read = |path: &Path| image::open(path).unwrap().to_rgba8().into_raw();
    for (name, suite) in [
        (
            "frames",
            crate::visual_frame_suite
                as fn(&Path, TileGraphicsDepth, &Path) -> std::io::Result<Vec<_>>,
        ),
        ("routes", crate::visual_route_suite),
    ] {
        let out = scratch(name);
        let reports = suite(&game_dir, TileGraphicsDepth::Ega16, &out).unwrap();
        let play: Vec<_> = reports
            .iter()
            .filter(|report| UVec2::new(report.width, report.height) == FRAME)
            .collect();
        assert!(!play.is_empty(), "{name}: no play frame");
        assert_frames_pass_untouched(
            play.iter()
                .map(|report| (report.label.clone(), report.byte_hash, read(&report.path))),
        );
        // M2-03: the brightness step leaves every faithful frame alone too.
        assert_frames_pass_untouched_by(
            play.iter()
                .map(|report| (report.label.clone(), report.byte_hash, read(&report.path))),
            brightness_step,
            brightened,
        );
        let _ = std::fs::remove_dir_all(out);
    }
    let _ = std::fs::remove_dir_all(root);
}

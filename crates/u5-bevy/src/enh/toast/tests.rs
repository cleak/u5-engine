//! Toasts and badges: replacement, timing under manual time, and the strip
//! (design 1.3.7, plan M0-13).

use u5_runtime::enhancements::TOGGLES;
use u5_runtime::{CH_FONT_LEN, IBM_CH_FILE, parse_ch_font};

use super::*;
use crate::enh::overlay::tests::idle_visual_state;
use crate::enh::{VisualLaunchConfig, add_enhancements, configure_enh_sets};

const FRAME: Duration = Duration::from_micros(16_667);
const REFUSED: &str = "A test refusal.";
const SAVED: &str = "A test confirmation.";
const WRITE_FAILED: &str = "A test write error.";

/// An app with the shell services, a play frame and, with `window`, a
/// window of that size. Real time only: virtual time never moves.
fn app(window: Option<Vec2>) -> App {
    let mut images = Assets::<Image>::default();
    let visual = idle_visual_state(&mut images);
    let mut app = App::new();
    app.insert_resource(Time::<()>::default())
        .insert_resource(Time::<Real>::default())
        .insert_resource(images)
        .insert_resource(visual);
    configure_enh_sets(&mut app);
    install_shell_services(&mut app);
    if let Some(size) = window {
        let mut window = Window::default();
        window.resolution.set(size.x, size.y);
        app.world_mut().spawn(window);
    }
    app
}

fn frame(app: &mut App) {
    app.world_mut()
        .resource_mut::<Time<Real>>()
        .update_with_duration(FRAME);
    app.update();
}

fn toasts(app: &mut App) -> Mut<'_, Toasts> {
    app.world_mut().resource_mut::<Toasts>()
}

fn showing(app: &App) -> Option<&'static str> {
    app.world().resource::<Toasts>().current()
}

/// Frames on which the toast posted before the first one shows.
fn frames_shown(app: &mut App) -> usize {
    let mut frames = 0;
    while showing(app).is_some() {
        frame(app);
        if showing(app).is_some() {
            frames += 1;
        }
        assert!(frames < 10_000, "the toast never expired");
    }
    frames
}

fn strips(app: &mut App) -> Vec<(&'static str, Node)> {
    let mut query = app.world_mut().query::<(&ToastStrip, &Node)>();
    query
        .iter(app.world())
        .map(|(strip, node)| (strip.text, node.clone()))
        .collect()
}

#[test]
fn no_toast_or_badge_unless_a_feature_installs_them() {
    let mut launch = VisualLaunchConfig::classic();
    for spec in TOGGLES {
        launch.enhancements.toggles.set(spec.toggle, true);
    }
    for launch in [VisualLaunchConfig::classic(), launch] {
        let mut app = App::new();
        add_enhancements(&mut app, &launch);
        assert!(!app.world().contains_resource::<Toasts>());
        assert!(!app.world().contains_resource::<ShellBadges>());
    }
}

#[test]
fn installing_twice_adds_the_services_once() {
    let mut app = app(None);
    install_shell_services(&mut app);
    let systems: usize = app
        .world_mut()
        .resource_mut::<bevy::ecs::schedule::Schedules>()
        .iter()
        .map(|(_, schedule)| {
            schedule
                .graph()
                .systems()
                .filter(|(_, system, _)| system.name().contains("tick_toasts"))
                .count()
        })
        .sum();
    assert_eq!(systems, 1);
    assert_eq!(
        *app.world().resource::<ShellBadges>(),
        ShellBadges::default()
    );
    assert!(!ShellBadges::default().any());
    assert!(
        ShellBadges {
            auto: true,
            modified: false
        }
        .any()
    );
}

#[test]
fn the_newest_toast_replaces_the_one_showing() {
    let mut toasts = Toasts::default();
    assert_eq!(toasts.current(), None);
    toasts.note(WRITE_FAILED);
    assert_eq!(toasts.current(), Some(WRITE_FAILED));
    assert_eq!(toasts.kind(), Some(ToastKind::Note));
    toasts.short(REFUSED);
    assert_eq!(
        toasts.current(),
        Some(REFUSED),
        "a short one replaces a note"
    );
    assert_eq!(toasts.kind(), Some(ToastKind::Short));
    toasts.note(WRITE_FAILED);
    assert_eq!(toasts.current(), Some(WRITE_FAILED));
    assert_eq!(toasts.kind(), Some(ToastKind::Note));
}

/// 2 s for refusals and confirmations, 5 s for notes, counted in real
/// frames under manual time.
#[test]
fn a_toast_shows_for_its_time_under_manual_time() {
    assert_eq!(ToastKind::Short.duration(), Duration::from_secs(2));
    assert_eq!(ToastKind::Note.duration(), Duration::from_secs(5));
    let mut app = app(None);
    frame(&mut app);

    toasts(&mut app).short(REFUSED);
    // The posting frame, then 119 more: 120 frames of 16.667 ms is the
    // first count to reach 2 s.
    assert_eq!(frames_shown(&mut app), 120);
    toasts(&mut app).note(WRITE_FAILED);
    assert_eq!(frames_shown(&mut app), 300);

    // The posting frame shows it whatever its delta.
    toasts(&mut app).short(SAVED);
    app.world_mut()
        .resource_mut::<Time<Real>>()
        .update_with_duration(Duration::from_secs(10));
    app.update();
    assert_eq!(showing(&app), Some(SAVED));
    frame(&mut app);
    assert_eq!(showing(&app), Some(SAVED));
}

/// A toast posted while one shows replaces it and starts its own time, and
/// the same text posted again starts again.
#[test]
fn a_replacing_toast_starts_its_own_time() {
    let mut app = app(None);
    frame(&mut app);
    toasts(&mut app).note(WRITE_FAILED);
    for _ in 0..200 {
        frame(&mut app);
    }
    assert_eq!(showing(&app), Some(WRITE_FAILED));
    toasts(&mut app).short(REFUSED);
    assert_eq!(frames_shown(&mut app), 120);

    toasts(&mut app).short(SAVED);
    for _ in 0..100 {
        frame(&mut app);
    }
    toasts(&mut app).short(SAVED);
    assert_eq!(frames_shown(&mut app), 120, "posted again, timed again");
}

/// A stopped game clock does not hold a toast: it ages in real time. The
/// test app never moves the default clock.
#[test]
fn a_toast_ages_in_real_time() {
    let mut app = app(None);
    toasts(&mut app).short(REFUSED);
    frame(&mut app);
    for _ in 0..119 {
        frame(&mut app);
    }
    assert_eq!(showing(&app), Some(REFUSED));
    frame(&mut app);
    assert_eq!(showing(&app), None);
    assert_eq!(app.world().resource::<Time>().elapsed(), Duration::ZERO);
}

fn assert_near(actual: Vec2, expected: Vec2, what: &str) {
    assert!(
        actual.abs_diff_eq(expected, 1e-3),
        "{what}: {actual} is not {expected}"
    );
}

#[test]
fn the_strip_sits_over_the_frames_top_row_without_a_margin() {
    // 4:3: the frame fills the window; a row is a 25th of its height.
    let rect = toast_strip_rect(Vec2::new(960.0, 720.0), None, 40);
    assert_near(rect.min, Vec2::ZERO, "4:3 corner");
    assert_near(rect.size(), Vec2::new(960.0, 28.8), "4:3 size");

    // 16:9: the 4:3 frame is centred; its 240-pixel bars are narrower than
    // the line, so the strip is centred on the frame.
    let rect = toast_strip_rect(Vec2::new(1920.0, 1080.0), None, 20);
    assert_near(rect.min, Vec2::new(600.0, 0.0), "16:9 corner");
    assert_near(rect.size(), Vec2::new(720.0, 43.2), "16:9 size");

    // Wider than the frame: shrunk to fit it, still one row's place.
    let rect = toast_strip_rect(Vec2::new(960.0, 720.0), None, 48);
    assert_near(rect.min, Vec2::ZERO, "long corner");
    assert_near(rect.size(), Vec2::new(960.0, 24.0), "long size");
}

#[test]
fn the_strip_moves_into_the_window_margin_when_one_exists() {
    // Taller than 4:3: the letterbox band above the frame holds a row.
    let rect = toast_strip_rect(Vec2::new(800.0, 800.0), None, 40);
    assert_near(rect.min, Vec2::new(0.0, 76.0), "band corner");
    assert_near(rect.size(), Vec2::new(800.0, 24.0), "band size");

    // A band thinner than a row is no margin.
    let rect = toast_strip_rect(Vec2::new(800.0, 620.0), None, 40);
    assert_near(rect.min, Vec2::new(0.0, 10.0), "thin band corner");

    // 32:9: a 1200-pixel bar holds a 20-cell line (720 pixels) whole, at
    // the frame's cell size, level with the frame's top.
    let rect = toast_strip_rect(Vec2::new(3840.0, 1080.0), None, 20);
    assert_near(rect.min, Vec2::new(240.0, 0.0), "bar corner");
    assert_near(rect.size(), Vec2::new(720.0, 43.2), "bar size");
}

#[test]
fn the_strip_follows_the_side_layout_and_keeps_out_of_panels() {
    use crate::enh::side::{
        PanelClaim, PanelKind, SidePreference, SideSpaceSettings, solve_side_space,
    };
    let window = Vec2::new(3840.0, 1080.0);
    let claim = |kind, side| PanelClaim { kind, side };
    let solution = solve_side_space(
        window,
        UVec2::new(VISUAL_PLAY_FRAME_WIDTH, VISUAL_PLAY_FRAME_HEIGHT),
        &[claim(PanelKind::Journal, SidePreference::Left)],
        &SideSpaceSettings::default(),
    );
    let layout = solution.layout;
    let frame = layout.frame.expect("the journal fits at 32:9");
    // The left bar holds the panel; the right bar takes the toast.
    let rect = toast_strip_rect(window, Some(&layout), 20);
    assert_near(
        rect.min,
        Vec2::new((frame.max.x + window.x - 720.0) / 2.0, frame.min.y),
        "right bar",
    );

    // Both bars hold panels and there is no band: over the frame.
    let solution = solve_side_space(
        window,
        UVec2::new(VISUAL_PLAY_FRAME_WIDTH, VISUAL_PLAY_FRAME_HEIGHT),
        &[
            claim(PanelKind::Journal, SidePreference::Left),
            claim(PanelKind::Conversation, SidePreference::Right),
        ],
        &SideSpaceSettings::default(),
    );
    let layout = solution.layout;
    let frame = layout.frame.expect("both fit at 32:9");
    let rect = toast_strip_rect(window, Some(&layout), 20);
    assert_near(
        rect.min,
        Vec2::new(frame.center().x - 360.0, frame.min.y),
        "over the frame",
    );
}

#[test]
fn the_strip_draws_the_text_in_the_frame_font() {
    let mut bytes = vec![0x00; CH_FONT_LEN];
    bytes[usize::from(b'A') * CH_CELL_SIDE] = 0x80;
    let font = parse_ch_font(&bytes, IBM_CH_FILE).unwrap();
    let (width, height, rgba) = toast_strip_rgba(&font, " A");
    assert_eq!((width, height), (16, 8));
    assert_eq!(rgba.len(), 16 * 8 * 4);
    let pixel = |x: usize, y: usize| {
        let offset = (y * 16 + x) * 4;
        <[u8; 4]>::try_from(&rgba[offset..offset + 4]).unwrap()
    };
    assert_eq!(pixel(8, 0), TOAST_INK, "the A's top-left dot");
    assert_eq!(pixel(9, 0), TOAST_PAPER);
    assert_eq!(pixel(0, 0), TOAST_PAPER, "the space");
    assert_eq!(pixel(8, 1), TOAST_PAPER);
}

/// The toast is its own UI node; the frame image is never written, and the
/// node goes when the toast does.
#[test]
fn the_toast_draws_as_a_node_and_never_writes_the_frame() {
    let mut app = app(Some(Vec2::new(1920.0, 1080.0)));
    frame(&mut app);
    let handle = app.world().resource::<VisualState>().image_handle.clone();
    let frame_before = app
        .world()
        .resource::<Assets<Image>>()
        .get(&handle)
        .unwrap()
        .data
        .clone();
    assert!(strips(&mut app).is_empty());

    toasts(&mut app).short(REFUSED);
    frame(&mut app);
    let drawn = strips(&mut app);
    assert_eq!(drawn.len(), 1);
    assert_eq!(drawn[0].0, REFUSED);
    let rect = toast_strip_rect(Vec2::new(1920.0, 1080.0), None, REFUSED.len());
    assert_eq!(drawn[0].1.left, Val::Px(rect.min.x));
    assert_eq!(drawn[0].1.top, Val::Px(rect.min.y));
    assert_eq!(drawn[0].1.width, Val::Px(rect.width()));
    assert_eq!(drawn[0].1.position_type, PositionType::Absolute);

    toasts(&mut app).note(WRITE_FAILED);
    frame(&mut app);
    let drawn = strips(&mut app);
    assert_eq!(drawn.len(), 1, "the newest replaces it");
    assert_eq!(drawn[0].0, WRITE_FAILED);

    while showing(&app).is_some() {
        frame(&mut app);
    }
    frame(&mut app);
    assert!(strips(&mut app).is_empty());
    let frame_after = app
        .world()
        .resource::<Assets<Image>>()
        .get(&handle)
        .unwrap()
        .data
        .clone();
    assert_eq!(frame_before, frame_after);
}

/// No window or no play frame: nothing to draw on, nothing spawned.
#[test]
fn nothing_draws_without_a_window() {
    let mut app = app(None);
    toasts(&mut app).short(REFUSED);
    frame(&mut app);
    assert!(strips(&mut app).is_empty());
    assert_eq!(showing(&app), Some(REFUSED));
}

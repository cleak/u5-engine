//! Toasts and badges: replacement, timing under manual time, and the strip
//! (design 1.3.7, plan M0-13).

use u5_runtime::enhancements::{TOGGLES, Toggle};
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

fn showing(app: &App) -> Option<String> {
    app.world()
        .resource::<Toasts>()
        .current()
        .map(str::to_string)
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

fn strips(app: &mut App) -> Vec<(String, Node)> {
    let mut query = app.world_mut().query::<(&ToastStrip, &Node)>();
    query
        .iter(app.world())
        .map(|(strip, node)| (strip.text.clone(), node.clone()))
        .collect()
}

#[test]
fn no_toast_or_badge_unless_a_feature_installs_them() {
    let mut launch = VisualLaunchConfig::classic();
    // ...except the grow-left layout, whose `[grow-left compose]` step,
    // frame fit and canvas size are its own (M1-09, display/grow tests),
    // the two menu toggles, whose overlay brings the shell services
    // (overlay tests), and auto-combat, which posts toasts and sets the
    // badge (below).
    for spec in TOGGLES.iter().filter(|spec| {
        spec.toggle != Toggle::DisplayGrowLeft
            && spec.toggle != Toggle::CombatAuto
            && !crate::enh::overlay::tests::MENU_OPENERS.contains(&spec.toggle)
            // The cheats page, an overlay feature, whose refusals are
            // toasts (M8-02).
            && !spec.toggle.key().starts_with("cheats.")
    }) {
        launch.enhancements.toggles.set(spec.toggle, true);
    }
    for launch in [VisualLaunchConfig::classic(), launch] {
        let mut app = App::new();
        add_enhancements(&mut app, &launch);
        assert!(!app.world().contains_resource::<Toasts>());
        assert!(!app.world().contains_resource::<ShellBadges>());
    }
    // Auto-combat installs them (M7-05).
    let mut auto = VisualLaunchConfig::classic();
    auto.enhancements.toggles.set(Toggle::CombatAuto, true);
    let mut app = App::new();
    add_enhancements(&mut app, &auto);
    assert!(app.world().contains_resource::<Toasts>());
    assert!(app.world().contains_resource::<ShellBadges>());
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

/// D-M4.7: a toast that carries a name is a `String` filled from a
/// `&'static str` template; it times and replaces like any other.
#[test]
fn a_toast_with_a_name_is_built_from_a_template() {
    use crate::menu::text::{SAVED_TO, SLOT_WRITE_FAILED, fill};
    let mut line = Toasts::default();
    line.short_text(fill(SAVED_TO, &["Despise"]));
    assert_eq!(line.current(), Some("Saved to \"Despise\"."));
    assert_eq!(line.kind(), Some(ToastKind::Short));
    line.note_text(fill(SLOT_WRITE_FAILED, &["Despise", "disk full"]));
    assert_eq!(
        line.current(),
        Some("Saved the game, but could not write \"Despise\": disk full.")
    );
    assert_eq!(line.kind(), Some(ToastKind::Note));
    line.short(REFUSED);
    assert_eq!(line.current(), Some(REFUSED), "a constant replaces it");

    let mut app = app(Some(Vec2::new(1920.0, 1080.0)));
    frame(&mut app);
    toasts(&mut app).short_text(fill(SAVED_TO, &["quick"]));
    frame(&mut app);
    let drawn = strips(&mut app);
    assert_eq!(drawn.len(), 1);
    assert_eq!(drawn[0].0, "Saved to \"quick\".");
    assert_eq!(frames_shown(&mut app), 119, "2 s, as a constant's");
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
    assert_eq!(showing(&app).as_deref(), Some(SAVED));
    frame(&mut app);
    assert_eq!(showing(&app).as_deref(), Some(SAVED));
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
    assert_eq!(showing(&app).as_deref(), Some(WRITE_FAILED));
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
    assert_eq!(showing(&app).as_deref(), Some(REFUSED));
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

/// Both badges' strip: a band cell, `>AUTO<`, a band cell, `>MOD<`, a
/// band cell.
const BOTH: u32 = 14;

fn rect_near(actual: Rect, min: Vec2, max: Vec2, what: &str) {
    assert_near(actual.min, min, what);
    assert_near(actual.max, max, what);
}

#[test]
fn words_wrap_at_spaces_and_a_long_word_refuses() {
    assert_eq!(
        wrap_words("Auto off: Avatar is hurt", 11).unwrap(),
        ["Auto off:", "Avatar is", "hurt"]
    );
    assert_eq!(
        wrap_words("A test refusal.", 40).unwrap(),
        ["A test refusal."]
    );
    assert_eq!(wrap_words("Ctrl+Shift+A", 11), None);
    assert_eq!(badge_cells(&[BADGE_AUTO, BADGE_MODIFIED]), BOTH);
    assert_eq!(badge_cells(&[]), 0);
}

/// 4:3: the frame fills the window, so there is no margin. The toast and
/// the badges sit over the map, a cell in from its rules (cells 1-22):
/// the toast at the bottom right, the badges at the top right, in the
/// frame's own cells (3 by 3.6 pixels here). Nothing covers the border.
#[test]
fn without_a_margin_they_sit_over_the_map_inside_its_rules() {
    let window = Vec2::new(960.0, 720.0);
    let placed = shell_placement(window, None, None, Some(REFUSED), BOTH);
    assert_near(placed.glyph, Vec2::new(3.0, 3.6), "the frame's pixel");
    let (lines, toast) = placed.toast.unwrap();
    assert_eq!(lines, [REFUSED]);
    rect_near(
        toast,
        Vec2::new(7.0 * 24.0, 21.0 * 28.8),
        Vec2::new(22.0 * 24.0, 22.0 * 28.8),
        "toast",
    );
    rect_near(
        placed.badges.unwrap(),
        Vec2::new(8.0 * 24.0, 2.0 * 28.8),
        Vec2::new(22.0 * 24.0, 3.0 * 28.8),
        "badges",
    );
    // Inside the map's rules (x and y 7 and 184 of the frame) with a
    // cell to spare.
    let inner = Rect::new(16.0 * 3.0, 16.0 * 3.6, 176.0 * 3.0, 176.0 * 3.6);
    for rect in [toast, placed.badges.unwrap()] {
        assert!(
            inner.contains(rect.min) && inner.contains(rect.max),
            "{rect:?}"
        );
    }

    // A long line wraps at spaces into the map's 20 cells.
    let placed = shell_placement(
        window,
        None,
        None,
        Some(crate::enh::auto_combat::AUTO_READY),
        0,
    );
    let (lines, toast) = placed.toast.unwrap();
    assert!(
        lines.len() > 1 && lines.iter().all(|line| line.len() <= 20),
        "{lines:?}"
    );
    assert!(
        inner.contains(toast.min) && inner.contains(toast.max),
        "{toast:?}"
    );
}

/// Taller than 4:3: the badges in the band above, the toast in the band
/// below, right-aligned a cell in from the frame, at a whole multiple of
/// the font (2 here, the frame being 2.5 across).
#[test]
fn a_band_holds_them_above_and_below_the_frame() {
    let placed = shell_placement(Vec2::new(800.0, 800.0), None, None, Some(REFUSED), BOTH);
    assert_near(placed.glyph, Vec2::new(2.0, 2.4), "whole pixels across");
    let (lines, toast) = placed.toast.unwrap();
    assert_eq!(lines, [REFUSED]);
    rect_near(
        toast,
        Vec2::new(784.0 - 15.0 * 16.0, 700.0 + (100.0 - 19.2) / 2.0),
        Vec2::new(784.0, 700.0 + (100.0 + 19.2) / 2.0),
        "toast",
    );
    rect_near(
        placed.badges.unwrap(),
        Vec2::new(784.0 - 14.0 * 16.0, (100.0 - 19.2) / 2.0),
        Vec2::new(784.0, (100.0 + 19.2) / 2.0),
        "badges",
    );
}

/// 16:9: the 240-pixel bar right of the frame takes the toast, a cell
/// from the frame and a line up from its bottom, at the largest whole
/// scale at which it wraps into three lines or fewer (3 here; the frame is
/// 4.5 across).
#[test]
fn a_free_side_bar_holds_the_toast_wrapped() {
    let placed = shell_placement(Vec2::new(1920.0, 1080.0), None, None, Some(REFUSED), 0);
    assert_near(placed.glyph, Vec2::new(3.0, 3.6), "whole pixels across");
    let (lines, toast) = placed.toast.unwrap();
    assert_eq!(lines, ["A test", "refusal."]);
    rect_near(
        toast,
        Vec2::new(1704.0, 1080.0 - 28.8 * 3.0),
        Vec2::new(1704.0 + 8.0 * 24.0, 1080.0 - 28.8),
        "toast",
    );
    assert!(toast.min.x >= 1680.0, "clear of the frame");
}

#[test]
fn the_toast_keeps_out_of_side_panels() {
    use crate::enh::side::{
        PanelClaim, PanelKind, SidePreference, SideSpaceSettings, solve_side_space,
    };
    let window = Vec2::new(3840.0, 1080.0);
    let claim = |kind, side| PanelClaim { kind, side };
    let frame_size = UVec2::new(VISUAL_PLAY_FRAME_WIDTH, VISUAL_PLAY_FRAME_HEIGHT);
    // The right bar holds a panel; the left bar takes the toast.
    let solution = solve_side_space(
        window,
        frame_size,
        &[claim(PanelKind::Journal, SidePreference::Right)],
        &SideSpaceSettings::default(),
    );
    let layout = solution.layout;
    let frame = layout.frame.expect("the journal fits at 32:9");
    let (_, toast) = shell_placement(window, Some(&layout), None, Some(REFUSED), 0)
        .toast
        .unwrap();
    assert!(toast.max.x <= frame.min.x, "{toast:?} left of {frame:?}");

    // Both bars hold panels and there is no band: over the map.
    let solution = solve_side_space(
        window,
        frame_size,
        &[
            claim(PanelKind::Journal, SidePreference::Left),
            claim(PanelKind::Conversation, SidePreference::Right),
        ],
        &SideSpaceSettings::default(),
    );
    let layout = solution.layout;
    let frame = layout.frame.expect("both fit at 32:9");
    let (_, toast) = shell_placement(window, Some(&layout), None, Some(REFUSED), 0)
        .toast
        .unwrap();
    assert!(frame.contains(toast.min) && frame.contains(toast.max));
    assert!(toast.min.y > frame.min.y + 8.0 * frame.height() / 200.0);
}

/// A grow-left canvas that fills the window (512x240 at 5 by 6): over the
/// map window, a cell in from its rules and right of the column's rule and
/// the crawl (cell 19), clear of the ribbon's top band and the wind tab's
/// bottom one, on the canvas's own cells.
#[test]
fn on_a_canvas_they_keep_clear_of_the_column_crawl_ribbon_and_tabs() {
    let window = Vec2::new(2560.0, 1440.0);
    let canvas = Rect::from_corners(Vec2::ZERO, window);
    let placed = shell_placement(
        window,
        None,
        Some((canvas, UVec2::new(512, 240))),
        Some(REFUSED),
        BOTH,
    );
    let (_, toast) = placed.toast.unwrap();
    rect_near(
        toast,
        Vec2::new(47.0 * 40.0, 27.0 * 48.0),
        Vec2::new(62.0 * 40.0, 28.0 * 48.0),
        "toast",
    );
    rect_near(
        placed.badges.unwrap(),
        Vec2::new(48.0 * 40.0, 3.0 * 48.0),
        Vec2::new(62.0 * 40.0, 4.0 * 48.0),
        "badges",
    );
    // The longest line still clears the column and crawl.
    let (_, toast) = shell_placement(
        window,
        None,
        Some((canvas, UVec2::new(384, 240))),
        Some(crate::enh::auto_combat::AUTO_READY),
        0,
    )
    .toast
    .unwrap();
    assert!(
        toast.min.x >= 20.0 * 8.0 * (2560.0 / 384.0) - 1e-3,
        "{toast:?}"
    );
}

/// The badges are their own node, in the chrome's blue with the words in
/// the frame's font; with no badge set there is no node.
#[test]
fn the_badges_draw_as_their_own_node() {
    let mut app = app(Some(Vec2::new(1920.0, 1080.0)));
    frame(&mut app);
    let badges = |app: &mut App| {
        let mut query = app.world_mut().query::<(&BadgeStrip, &Node)>();
        query.iter(app.world()).count()
    };
    assert_eq!(badges(&mut app), 0);
    app.world_mut().resource_mut::<ShellBadges>().auto = true;
    frame(&mut app);
    assert_eq!(badges(&mut app), 1);
    app.world_mut().resource_mut::<ShellBadges>().modified = true;
    frame(&mut app);
    assert_eq!(badges(&mut app), 1, "one strip, rebuilt");
    *app.world_mut().resource_mut::<ShellBadges>() = ShellBadges::default();
    frame(&mut app);
    assert_eq!(badges(&mut app), 0);

    let mut bytes = vec![0x00; CH_FONT_LEN];
    bytes[usize::from(b'A') * CH_CELL_SIDE] = 0x80;
    let font = parse_ch_font(&bytes, IBM_CH_FILE).unwrap();
    let (w, h, rgba) = badge_rgba(&font, &[BADGE_AUTO]);
    assert_eq!((w, h), (8 * 8, 8));
    let pixel = |x: u32, y: u32| {
        let i = ((y * w + x) * 4) as usize;
        <[u8; 4]>::try_from(&rgba[i..i + 4]).unwrap()
    };
    assert_eq!(
        pixel(0, 4),
        crate::menu::frame::ega(crate::menu::frame::BAND)
    );
    assert_eq!(
        pixel(16, 0),
        crate::menu::frame::ega(crate::menu::frame::INK),
        "the A's dot"
    );
    assert_eq!(
        pixel(17, 0),
        crate::menu::frame::ega(crate::menu::frame::PAPER)
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
    let (_, rect) = shell_placement(Vec2::new(1920.0, 1080.0), None, None, Some(REFUSED), 0)
        .toast
        .unwrap();
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
    assert_eq!(showing(&app).as_deref(), Some(REFUSED));
}

/// Each finds its own place: at 16:9 the bar holds the toast but not both
/// badges side by side, so the badges go over the map while the toast
/// stays in the bar.
#[test]
fn the_toast_and_the_badges_each_take_the_best_place() {
    let placed = shell_placements(Vec2::new(1706.0, 960.0), None, None, Some(REFUSED), BOTH);
    let (_, toast) = placed.toast.unwrap();
    let badges = placed.badges.unwrap();
    assert!(toast.min.x >= 1493.0, "the toast in the bar: {toast:?}");
    assert!(
        badges.max.x <= 213.0 + 184.0 * 4.0,
        "the badges over the map: {badges:?}"
    );
    assert!(toast.intersect(badges).is_empty());
}

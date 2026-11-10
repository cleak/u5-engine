//! The side-space arbiter and the panel host: zero footprint, the frame
//! size, arbiter geometry at five aspects with the guard and priority
//! drops, and the cell-panel renderer (design 1.3.8, plan M0-14).

use std::sync::{Arc, Mutex};

use u5_runtime::enhancements::{
    LockReason, ResolvedEnhancements, SettingValue, SettingValues, TOGGLES, Toggle,
};
use u5_runtime::{CH_FONT_LEN, IBM_CH_FILE, parse_ch_font};

use super::*;
use crate::enh::overlay::tests::idle_visual_state;
use crate::enh::{VisualLaunchConfig, add_enhancements, configure_enh_sets};
use crate::fit_display_to_window;

const FRAME: UVec2 = UVec2::new(VISUAL_PLAY_FRAME_WIDTH, VISUAL_PLAY_FRAME_HEIGHT);
const PANEL_TITLE: &str = "TEST PANEL";

/// The five window shapes of the plan, at 1080 or 768 rows.
const ASPECT_4_3: Vec2 = Vec2::new(1024.0, 768.0);
const ASPECT_16_10: Vec2 = Vec2::new(1920.0, 1200.0);
const ASPECT_16_9: Vec2 = Vec2::new(1920.0, 1080.0);
const ASPECT_21_9: Vec2 = Vec2::new(2560.0, 1080.0);
const ASPECT_32_9: Vec2 = Vec2::new(3840.0, 1080.0);
const ASPECTS: [Vec2; 5] = [
    ASPECT_4_3,
    ASPECT_16_10,
    ASPECT_16_9,
    ASPECT_21_9,
    ASPECT_32_9,
];

fn claim(kind: PanelKind, side: SidePreference) -> PanelClaim {
    PanelClaim { kind, side }
}

fn both_auto() -> [PanelClaim; 2] {
    [
        claim(PanelKind::Journal, SidePreference::Auto),
        claim(PanelKind::Conversation, SidePreference::Auto),
    ]
}

fn solve(window: Vec2, claims: &[PanelClaim]) -> SideSolution {
    solve_side_space(window, FRAME, claims, &SideSpaceSettings::default())
}

fn with_guard(percent: u8) -> SideSpaceSettings {
    SideSpaceSettings {
        guard_percent: percent,
        ..SideSpaceSettings::default()
    }
}

fn granted(solution: &SideSolution) -> Vec<(PanelKind, PanelSide)> {
    solution
        .layout
        .granted
        .iter()
        .map(|(kind, _)| (*kind, solution.layout.side_of(*kind).unwrap()))
        .collect()
}

fn assert_near(actual: Vec2, expected: Vec2, what: &str) {
    assert!(
        actual.abs_diff_eq(expected, 1e-2),
        "{what}: {actual} is not {expected}"
    );
}

/// The frame scale with `panels` cell panels beside it.
fn scale(window: Vec2, panels: u32) -> f32 {
    (window.x / (320 + 136 * panels) as f32).min(window.y / 240.0)
}

/// Checks the layout's frame and panels against scale `u` with the frame's
/// left edge at `left`.
fn assert_geometry(solution: &SideSolution, window: Vec2, u: f32, left: f32, what: &str) {
    let layout = &solution.layout;
    let frame = layout.frame.expect("a panel is granted");
    let top = (window.y - 240.0 * u) / 2.0;
    assert_near(frame.min, Vec2::new(left, top), what);
    assert_near(frame.size(), Vec2::new(320.0 * u, 240.0 * u), what);
    for (kind, rect) in &layout.granted {
        assert_near(rect.size(), Vec2::new(136.0 * u, 240.0 * u), what);
        assert_eq!(rect.min.y, frame.min.y, "{what}: {kind:?} level with frame");
        let (edge, against) = match layout.side_of(*kind).unwrap() {
            PanelSide::Left => (rect.max.x, frame.min.x),
            PanelSide::Right => (rect.min.x, frame.max.x),
        };
        assert!(
            (edge - against).abs() < 1e-2,
            "{what}: {kind:?} against frame"
        );
        assert!(
            rect.min.x >= -1e-3 && rect.max.x <= window.x + 1e-3,
            "{what}"
        );
    }
    assert_eq!(layout.left.is_some(), frame.min.x > 0.0, "{what}: left bar");
    assert_eq!(layout.right.is_some(), frame.max.x < window.x, "{what}");
}

// ---------------------------------------------------------------------------
// Zero footprint.

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

#[test]
fn no_side_space_while_no_side_feature_is_on() {
    for launch in [
        VisualLaunchConfig::classic(),
        VisualLaunchConfig {
            enhancements: ResolvedEnhancements::locked(LockReason::CliFlag),
            ..VisualLaunchConfig::classic()
        },
    ] {
        let mut app = App::new();
        add_enhancements(&mut app, &launch);
        assert!(!app.world().contains_resource::<SideLayout>());
    }
    // Every catalogue toggle on: none of them claims side space yet.
    let mut launch = VisualLaunchConfig::classic();
    // ...except the grow-left layout, whose `[grow-left compose]` step,
    // frame fit and canvas size are its own (M1-09, display/grow tests).
    for spec in TOGGLES
        .iter()
        .filter(|spec| spec.toggle != Toggle::DisplayGrowLeft)
    {
        launch.enhancements.toggles.set(spec.toggle, true);
    }
    assert!(!side_claim_on(launch.enhancements.toggles));
    let mut app = App::new();
    add_enhancements(&mut app, &launch);
    assert!(app.is_plugin_added::<crate::enh::EnhancementsPlugin>());
    let world = app.world();
    assert!(!world.contains_resource::<PanelRegistry>());
    assert!(!world.contains_resource::<SideLayout>());
    assert!(!world.contains_resource::<SideSpaceSettings>());
    for system in [
        "layout_side_space",
        "fit_frame_to_side_layout",
        "draw_side_panels",
    ] {
        assert!(!has_system(&mut app, system), "{system}");
    }
    // The claiming toggles arrive with chapters 5 and 8.
    for key in SIDE_CLAIM_KEYS {
        assert_eq!(Toggle::from_key(key), None, "{key}");
    }
}

#[test]
fn installing_twice_adds_the_arbiter_once() {
    let mut app = App::new();
    install_side_space(&mut app, SideSpaceSettings::default());
    install_side_space(&mut app, with_guard(50));
    assert_eq!(
        app.world().resource::<SideSpaceSettings>().guard_percent,
        80
    );
    let schedules = app.world().resource::<bevy::ecs::schedule::Schedules>();
    let count = schedules
        .iter()
        .flat_map(|(_, schedule)| schedule.graph().systems())
        .filter(|(_, system, _)| system.name().contains("layout_side_space"))
        .count();
    assert_eq!(count, 1);
    assert!(
        app.world().contains_resource::<Toasts>(),
        "for the hidden toast"
    );
}

#[test]
fn the_faithful_fit_owns_the_frame_unless_a_panel_is_granted() {
    let mut app = App::new();
    let world = app.world_mut();
    let mut check = IntoSystem::into_system(faithful_fit_owns_frame);
    check.initialize(world);
    assert!(check.run((), world), "no side layout: Classic");
    world.insert_resource(SideLayout::default());
    assert!(check.run((), world), "nothing granted");
    let solution = solve(ASPECT_32_9, &both_auto());
    world.insert_resource(solution.layout);
    assert!(!check.run((), world), "a panel is granted");
}

// ---------------------------------------------------------------------------
// The solver.

#[test]
fn the_frame_is_unchanged_unless_a_panel_is_granted() {
    for window in ASPECTS {
        let solution = solve(window, &[]);
        assert_eq!(solution, SideSolution::default(), "{window}: no claim");
        let solution = solve(window, &[claim(PanelKind::Minimap, SidePreference::Auto)]);
        assert_eq!(solution, SideSolution::default(), "{window}: minimap only");
        assert!(!solution.layout.owns_frame());
    }
    // A granted panel with room to spare leaves the frame where the
    // faithful fit puts it, at its exact size.
    let solution = solve(
        ASPECT_32_9,
        &[claim(PanelKind::Journal, SidePreference::Auto)],
    );
    let frame = solution.layout.frame.unwrap();
    assert_near(
        frame.size(),
        fitted_display_size(
            VISUAL_PLAY_FRAME_WIDTH,
            VISUAL_PLAY_FRAME_HEIGHT,
            ASPECT_32_9,
        ),
        "unshrunk size",
    );
    assert_near(frame.center(), ASPECT_32_9 / 2.0, "unshrunk centre");
    // A minimised window arranges nothing and hides nothing.
    assert_eq!(solve(Vec2::ZERO, &both_auto()), SideSolution::default());
}

#[test]
fn arbiter_geometry_at_five_aspects() {
    // 4:3: no bars; even one panel would shrink the frame to 70 %.
    let solution = solve(ASPECT_4_3, &both_auto());
    assert!(!solution.layout.owns_frame(), "4:3");
    assert_eq!(
        solution.hidden,
        [PanelKind::Journal, PanelKind::Conversation]
    );

    // 16:10 and 16:9: one panel fits within the 80 % guard, two do not;
    // the lower priority one is hidden. The frame shrinks and the group is
    // centred.
    for window in [ASPECT_16_10, ASPECT_16_9] {
        let solution = solve(window, &both_auto());
        assert_eq!(granted(&solution), [(PanelKind::Journal, PanelSide::Right)]);
        assert_eq!(solution.hidden, [PanelKind::Conversation]);
        let u = scale(window, 1);
        assert!(u >= 0.8 * scale(window, 0) && u < scale(window, 0));
        assert!(scale(window, 2) < 0.8 * scale(window, 0));
        assert_geometry(&solution, window, u, (window.x - 456.0 * u) / 2.0, "16:x");
    }

    // 21:9: both fit within the guard, shrunk, packed and centred.
    let solution = solve(ASPECT_21_9, &both_auto());
    assert_eq!(
        granted(&solution),
        [
            (PanelKind::Journal, PanelSide::Right),
            (PanelKind::Conversation, PanelSide::Left),
        ]
    );
    assert!(solution.hidden.is_empty());
    let u = scale(ASPECT_21_9, 2);
    assert!(u < scale(ASPECT_21_9, 0));
    let left = (ASPECT_21_9.x - 592.0 * u) / 2.0 + 136.0 * u;
    assert_geometry(&solution, ASPECT_21_9, u, left, "21:9");

    // 32:9: both fit in the bars at full size; the frame does not move.
    let solution = solve(ASPECT_32_9, &both_auto());
    assert_eq!(solution.layout.granted.len(), 2);
    let u = scale(ASPECT_32_9, 0);
    assert_eq!(u, 4.5);
    assert_geometry(&solution, ASPECT_32_9, u, 1200.0, "32:9");
}

#[test]
fn the_guard_drops_panels_below_its_percent() {
    // 16:9 with one panel keeps 93.6 % of the frame; two keep 72 %.
    let solution = solve_side_space(ASPECT_16_9, FRAME, &both_auto(), &with_guard(100));
    assert!(
        !solution.layout.owns_frame(),
        "100 %: only a free bar will do"
    );
    assert_eq!(
        solution.hidden,
        [PanelKind::Journal, PanelKind::Conversation]
    );
    let solution = solve_side_space(ASPECT_16_9, FRAME, &both_auto(), &with_guard(94));
    assert!(solution.layout.granted.is_empty());
    let solution = solve_side_space(ASPECT_16_9, FRAME, &both_auto(), &with_guard(93));
    assert_eq!(solution.layout.granted.len(), 1);
    let solution = solve_side_space(ASPECT_16_9, FRAME, &both_auto(), &with_guard(72));
    assert_eq!(solution.layout.granted.len(), 2);
    assert!(solution.hidden.is_empty());
    let solution = solve_side_space(ASPECT_16_9, FRAME, &both_auto(), &with_guard(73));
    assert_eq!(solution.layout.granted.len(), 1);
    // 100 % still grants a panel that fits in a free bar.
    let solution = solve_side_space(ASPECT_32_9, FRAME, &both_auto(), &with_guard(100));
    assert_eq!(solution.layout.granted.len(), 2);
}

#[test]
fn priority_decides_which_panel_is_dropped() {
    let conversation_first = SideSpaceSettings {
        priority: vec![
            PanelKind::Conversation,
            PanelKind::Journal,
            PanelKind::Minimap,
        ],
        ..SideSpaceSettings::default()
    };
    let solution = solve_side_space(ASPECT_16_9, FRAME, &both_auto(), &conversation_first);
    assert_eq!(
        granted(&solution),
        [(PanelKind::Conversation, PanelSide::Right)]
    );
    assert_eq!(solution.hidden, [PanelKind::Journal]);
}

#[test]
fn two_panels_on_one_side_go_by_priority() {
    // Both ask for the left: the journal (first) keeps it and the
    // conversation moves to the free side.
    let claims = [
        claim(PanelKind::Conversation, SidePreference::Left),
        claim(PanelKind::Journal, SidePreference::Left),
    ];
    let solution = solve(ASPECT_32_9, &claims);
    assert_eq!(
        granted(&solution),
        [
            (PanelKind::Journal, PanelSide::Left),
            (PanelKind::Conversation, PanelSide::Right),
        ]
    );
    // `auto` takes the side the other does not use, whatever its priority.
    let claims = [
        claim(PanelKind::Journal, SidePreference::Auto),
        claim(PanelKind::Conversation, SidePreference::Right),
    ];
    let solution = solve(ASPECT_32_9, &claims);
    assert_eq!(
        granted(&solution),
        [
            (PanelKind::Journal, PanelSide::Left),
            (PanelKind::Conversation, PanelSide::Right),
        ]
    );
    // One panel on its own side at full size: the frame stays centred.
    let solution = solve(
        ASPECT_32_9,
        &[claim(PanelKind::Conversation, SidePreference::Left)],
    );
    assert_eq!(
        granted(&solution),
        [(PanelKind::Conversation, PanelSide::Left)]
    );
    assert_geometry(&solution, ASPECT_32_9, 4.5, 1200.0, "one left");
}

#[test]
fn a_panel_that_fits_unshrunk_but_not_centred_moves_the_frame() {
    // 2052x1080: the full-size frame leaves 306-pixel bars, too narrow for
    // a 612-pixel panel, but frame and panel fit side by side at u0 = 4.5
    // once the frame moves over.
    let window = Vec2::new(2052.0, 1080.0);
    let solution = solve(window, &[claim(PanelKind::Journal, SidePreference::Auto)]);
    assert_eq!(scale(window, 1), 4.5);
    assert_geometry(&solution, window, 4.5, 0.0, "shifted");
}

#[test]
fn settings_come_from_the_resolved_values() {
    let classic = ResolvedEnhancements::classic();
    assert_eq!(
        SideSpaceSettings::from_values(&classic.values),
        SideSpaceSettings::default()
    );
    let defaults = SideSpaceSettings::default();
    assert_eq!(
        defaults.priority,
        [
            PanelKind::Journal,
            PanelKind::Conversation,
            PanelKind::Minimap
        ]
    );
    assert_eq!(defaults.guard_percent, 80);

    // A list that leaves a kind out places it after the ones it names.
    let words = ["conversation"];
    let settings = SideSpaceSettings::from_priority(words.into_iter(), 60);
    assert_eq!(
        settings.priority,
        [
            PanelKind::Conversation,
            PanelKind::Journal,
            PanelKind::Minimap
        ]
    );
    let words = ["history", "journal", "journal"];
    let settings = SideSpaceSettings::from_priority(words.into_iter(), 60);
    assert_eq!(settings.priority, defaults.priority);
    assert_eq!(settings.guard_percent, 60);

    let values: SettingValues = classic.values.clone();
    let lookup = |key: &str| values.get(key).cloned();
    assert_eq!(
        lookup("display.panel_priority"),
        Some(SettingValue::List(vec![
            "journal".to_string(),
            "conversation".to_string()
        ]))
    );
    assert_eq!(lookup("display.panel_guard"), Some(SettingValue::Int(80)));
    for kind in PanelKind::ALL {
        assert_eq!(
            kind.word().and_then(PanelKind::from_word),
            kind.word().map(|_| kind)
        );
    }
}

// ---------------------------------------------------------------------------
// The cell-panel renderer.

fn test_font() -> FixedCellFont {
    let mut bytes = vec![0x00; CH_FONT_LEN];
    bytes[usize::from(b'A') * CH_CELL_SIDE] = 0x80;
    bytes[usize::from(b'T') * CH_CELL_SIDE] = 0x80;
    parse_ch_font(&bytes, IBM_CH_FILE).unwrap()
}

fn pixel(rgba: &[u8], x: usize, y: usize) -> [u8; 4] {
    let offset = (y * PANEL_WIDTH as usize + x) * 4;
    <[u8; 4]>::try_from(&rgba[offset..offset + 4]).unwrap()
}

#[test]
fn the_cell_panel_is_136_by_200() {
    assert_eq!((PANEL_WIDTH, PANEL_HEIGHT), (136, 200));
    let rows = PanelRows::new("T", &[" A"]);
    let rgba = render_cell_panel(&test_font(), &rows, PanelSide::Right);
    assert_eq!(rgba.len(), 136 * 200 * 4);
}

#[test]
fn the_renderer_draws_title_and_rows_beside_the_gutter() {
    let font = test_font();
    let rows = PanelRows::new("T", &[" A"]);
    // A right panel: the gutter is on its left, next to the frame.
    let rgba = render_cell_panel(&font, &rows, PanelSide::Right);
    assert_eq!(pixel(&rgba, 8, 0), TITLE_INK, "the T, after the gutter");
    assert_eq!(pixel(&rgba, 8 + 8, 8), ROW_INK, "the A, row one, cell one");
    assert_eq!(pixel(&rgba, 0, 0), PAPER, "the gutter");
    assert_eq!(pixel(&rgba, 9, 0), PAPER);
    // A left panel: the gutter is on its right.
    let rgba = render_cell_panel(&font, &rows, PanelSide::Left);
    assert_eq!(pixel(&rgba, 0, 0), TITLE_INK);
    assert_eq!(pixel(&rgba, 8, 8), ROW_INK);
    for x in 128..136 {
        for y in 0..200 {
            assert_eq!(pixel(&rgba, x, y), PAPER, "gutter at {x},{y}");
        }
    }
}

#[test]
fn panel_rows_pad_and_cut() {
    let long = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let many: Vec<&str> = (0..30).map(|_| "A").collect();
    let rows = PanelRows::new(long, &many);
    assert_eq!(rows.title(), b"ABCDEFGHIJKLMNOP");
    assert_eq!(rows.row(0), b"A               ");
    assert_eq!(rows.row(23), b"A               ");
    let rows = PanelRows::new("", &[]);
    assert_eq!(rows.title(), &[b' '; 16]);
    assert_eq!(rows.row(23), &[b' '; 16]);
}

// ---------------------------------------------------------------------------
// In an app: the frame sprite, the panel host and the toast.

/// A test provider whose content the test sets.
#[derive(Clone)]
struct TestPanel {
    kind: PanelKind,
    side: SidePreference,
    content: Arc<Mutex<Option<PanelRows>>>,
}

impl SidePanelProvider for TestPanel {
    fn kind(&self) -> PanelKind {
        self.kind
    }

    fn side(&self) -> SidePreference {
        self.side
    }

    fn snapshot(&self, state: Option<&PlayState>) -> Option<PanelContent> {
        state?;
        self.content
            .lock()
            .unwrap()
            .clone()
            .map(PanelContent::Cells)
    }
}

impl TestPanel {
    fn new(kind: PanelKind, title: &str) -> Self {
        Self {
            kind,
            side: SidePreference::Auto,
            content: Arc::new(Mutex::new(Some(PanelRows::new(title, &[])))),
        }
    }

    fn show(&self, rows: Option<PanelRows>) {
        *self.content.lock().unwrap() = rows;
    }
}

/// The faithful fit as both chains register it, a window, a frame sprite
/// and a play frame; with `side`, the arbiter.
fn app(window: Vec2, side: bool) -> App {
    let mut images = Assets::<Image>::default();
    let mut visual = idle_visual_state(&mut images);
    visual.text_font = test_font();
    let mut app = App::new();
    app.insert_resource(Time::<()>::default())
        .insert_resource(Time::<Real>::default())
        .insert_resource(images)
        .insert_resource(visual);
    configure_enh_sets(&mut app);
    app.add_systems(
        Update,
        fit_display_to_window.run_if(faithful_fit_owns_frame),
    );
    if side {
        install_side_space(&mut app, SideSpaceSettings::default());
    }
    let mut w = Window::default();
    w.resolution.set(window.x, window.y);
    app.world_mut().spawn(w);
    let image = app.world().resource::<VisualState>().image_handle.clone();
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

fn frame(app: &mut App) {
    app.world_mut()
        .resource_mut::<Time<Real>>()
        .update_with_duration(std::time::Duration::from_millis(16));
    app.update();
}

fn sprite(app: &mut App) -> (Vec2, Vec2) {
    let mut query = app.world_mut().query::<(&Sprite, &Transform)>();
    let (sprite, transform) = query.single(app.world()).unwrap();
    (
        sprite.custom_size.unwrap(),
        transform.translation.truncate(),
    )
}

fn panels(app: &mut App) -> Vec<(PanelKind, Node, Handle<Image>)> {
    let mut query = app
        .world_mut()
        .query::<(&SidePanelNode, &Node, &ImageNode)>();
    query
        .iter(app.world())
        .map(|(panel, node, image)| (panel.kind, node.clone(), image.image.clone()))
        .collect()
}

fn register(app: &mut App, provider: TestPanel) {
    app.world_mut()
        .resource_mut::<PanelRegistry>()
        .register(provider);
}

#[test]
fn with_every_key_off_the_sprite_keeps_the_1_0_1_size() {
    for window in ASPECTS {
        let fitted = fitted_display_size(VISUAL_PLAY_FRAME_WIDTH, VISUAL_PLAY_FRAME_HEIGHT, window);
        for side in [false, true] {
            let mut app = app(window, side);
            for _ in 0..3 {
                frame(&mut app);
                assert_eq!(sprite(&mut app), (fitted, Vec2::ZERO), "{window} {side}");
            }
            assert!(panels(&mut app).is_empty());
        }
    }
}

#[test]
fn a_granted_panel_owns_the_frame_and_gives_it_back() {
    let window = ASPECT_16_9;
    let fitted = fitted_display_size(VISUAL_PLAY_FRAME_WIDTH, VISUAL_PLAY_FRAME_HEIGHT, window);
    let mut app = app(window, true);
    frame(&mut app);
    let journal = TestPanel::new(PanelKind::Journal, PANEL_TITLE);
    register(&mut app, journal.clone());
    frame(&mut app);

    let layout = app.world().resource::<SideLayout>().clone();
    let frame_rect = layout.frame.expect("16:9 grants one panel");
    let (size, centre) = sprite(&mut app);
    assert_eq!(size, frame_rect.size());
    assert_near(
        centre,
        Vec2::new(
            frame_rect.center().x - window.x / 2.0,
            window.y / 2.0 - frame_rect.center().y,
        ),
        "sprite centre",
    );
    assert!(size.x < fitted.x, "the frame shrank within the guard");
    // The faithful fit no longer runs, so it does not undo the layout.
    frame(&mut app);
    assert_eq!(sprite(&mut app).0, frame_rect.size());

    let drawn = panels(&mut app);
    assert_eq!(drawn.len(), 1);
    let rect = layout.granted[0].1;
    assert_eq!(drawn[0].0, PanelKind::Journal);
    assert_eq!(drawn[0].1.left, Val::Px(rect.min.x));
    assert_eq!(drawn[0].1.top, Val::Px(rect.min.y));
    assert_eq!(drawn[0].1.width, Val::Px(rect.width()));
    assert_eq!(drawn[0].1.height, Val::Px(rect.height()));

    // The feature goes off: the frame is the faithful one again at once.
    app.world_mut()
        .resource_mut::<PanelRegistry>()
        .remove(PanelKind::Journal);
    frame(&mut app);
    assert!(!app.world().resource::<SideLayout>().owns_frame());
    assert_eq!(sprite(&mut app), (fitted, Vec2::ZERO));
    assert!(panels(&mut app).is_empty());
    frame(&mut app);
    assert_eq!(sprite(&mut app), (fitted, Vec2::ZERO));
}

#[test]
fn the_panel_repaints_only_on_change_and_never_writes_the_frame() {
    let mut app = app(ASPECT_32_9, true);
    let handle = app.world().resource::<VisualState>().image_handle.clone();
    let frame_before = app
        .world()
        .resource::<Assets<Image>>()
        .get(&handle)
        .unwrap()
        .data
        .clone();
    let journal = TestPanel::new(PanelKind::Journal, PANEL_TITLE);
    register(&mut app, journal.clone());
    frame(&mut app);
    let drawn = panels(&mut app);
    assert_eq!(drawn.len(), 1);
    let image = drawn[0].2.clone();
    let images_before = app.world().resource::<Assets<Image>>().len();

    // Unchanged content: the same image, the same pixels.
    frame(&mut app);
    frame(&mut app);
    assert_eq!(panels(&mut app)[0].2, image);
    let pixels = app
        .world()
        .resource::<Assets<Image>>()
        .get(&image)
        .unwrap()
        .data
        .clone();

    // New cells: the same image, repainted in place.
    journal.show(Some(PanelRows::new("T", &["A"])));
    frame(&mut app);
    assert_eq!(panels(&mut app)[0].2, image);
    assert_eq!(app.world().resource::<Assets<Image>>().len(), images_before);
    let repainted = app
        .world()
        .resource::<Assets<Image>>()
        .get(&image)
        .unwrap()
        .data
        .clone();
    assert!(pixels != repainted, "the new cells are drawn");

    // Nothing to show: the slot is empty but kept.
    journal.show(None);
    frame(&mut app);
    assert!(panels(&mut app).is_empty());
    assert!(app.world().resource::<SideLayout>().owns_frame());

    let frame_after = app
        .world()
        .resource::<Assets<Image>>()
        .get(&handle)
        .unwrap()
        .data
        .clone();
    assert!(
        frame_before == frame_after,
        "the frame image is never written"
    );
}

#[test]
fn a_panel_with_no_room_is_hidden_with_one_toast() {
    let mut app = app(ASPECT_4_3, true);
    register(&mut app, TestPanel::new(PanelKind::Journal, PANEL_TITLE));
    frame(&mut app);
    assert!(!app.world().resource::<SideLayout>().owns_frame());
    assert!(panels(&mut app).is_empty());
    assert_eq!(
        app.world().resource::<Toasts>().current(),
        Some(PANEL_HIDDEN)
    );
    for _ in 0..200 {
        frame(&mut app);
    }
    assert_eq!(
        app.world().resource::<Toasts>().current(),
        None,
        "said once, not again"
    );
    let fitted = fitted_display_size(
        VISUAL_PLAY_FRAME_WIDTH,
        VISUAL_PLAY_FRAME_HEIGHT,
        ASPECT_4_3,
    );
    assert_eq!(sprite(&mut app), (fitted, Vec2::ZERO));
}

#[test]
fn panel_text_is_plain() {
    assert!(PANEL_HIDDEN.is_ascii());
    assert!(PANEL_HIDDEN.ends_with('.'));
}

//! The side fill (design 2.2.11, 2.3.4, P2 tests; build plan M1-04): bar
//! geometry from the solved layout, no node for a black fill, the ribbon
//! checked only for non-black bars (no art hash), the bezel crop and its
//! fallbacks, and the game left alone.

use bevy::window::WindowResolution;
use u5_runtime::enhancements::{
    CliSetting, HostCapabilities, LockState, PixelAspect, ResolveInput, ScaleMode,
};
use u5_runtime::test_fixtures::{open_grid, test_state};
use u5_runtime::{CH_FONT_LEN, IBM_CH_FILE, parse_ch_font};

use super::*;
use crate::display::DisplayPlugin;
use crate::display::scale::{ScaleFit, ScalePolicy};
use crate::enh::VisualLaunchConfig;
use crate::enh::equivalence::{Scene, Shell, Timeline, check_shell_equivalence};
use crate::enh::side::{FrameFit, SideSpaceSettings, WindowPixels, solve_frame, solve_side_space};

fn close(a: Rect, b: Rect) -> bool {
    (a.min - b.min).abs().max_element() < 1.0e-3 && (a.max - b.max).abs().max_element() < 1.0e-3
}

fn rect(x0: f32, y0: f32, x1: f32, y1: f32) -> Rect {
    Rect::new(x0, y0, x1, y1)
}

// --- Geometry. -------------------------------------------------------------

#[test]
fn the_bars_beside_the_1_0_1_fit() {
    // 16:9: the 4:3 frame is 1440 wide, centred.
    let geometry = fill_geometry(Vec2::new(1920.0, 1080.0), None).unwrap();
    assert!(close(geometry.frame, rect(240.0, 0.0, 1680.0, 1080.0)));
    assert!(close(geometry.left.unwrap(), rect(0.0, 0.0, 240.0, 1080.0)));
    assert!(close(
        geometry.right.unwrap(),
        rect(1680.0, 0.0, 1920.0, 1080.0)
    ));
    assert_eq!(geometry.frame_pixel(), Vec2::new(4.5, 5.4));

    // 21:9.
    let geometry = fill_geometry(Vec2::new(3440.0, 1440.0), None).unwrap();
    assert!(close(geometry.left.unwrap(), rect(0.0, 0.0, 760.0, 1440.0)));
    assert!(close(
        geometry.right.unwrap(),
        rect(2680.0, 0.0, 3440.0, 1440.0)
    ));

    // 4:3 and taller have no bars: letterbox bands are not pillars.
    for window in [Vec2::new(1600.0, 1200.0), Vec2::new(1280.0, 1024.0)] {
        let geometry = fill_geometry(window, None).unwrap();
        assert_eq!((geometry.left, geometry.right), (None, None), "{window}");
    }
    // A minimised window has nothing.
    assert_eq!(fill_geometry(Vec2::ZERO, None), None);
}

#[test]
fn the_bars_follow_the_solved_layout_while_it_owns_the_frame() {
    let window = WindowPixels::logical_at_one(Vec2::new(1920.0, 1080.0));
    let image = UVec2::new(320, 200);
    for mode in [
        ScaleMode::Sharp,
        ScaleMode::IntegerSquare,
        ScaleMode::IntegerHybrid,
    ] {
        let policy = ScaleFit::new(ScalePolicy::new(mode, PixelAspect::Dos));
        let solution = solve_frame(
            window,
            image,
            &[],
            &SideSpaceSettings::default(),
            Some(&policy as &dyn FrameFit),
        );
        let layout = solution.layout;
        let frame = layout.frame.expect("a policy places the frame");
        let geometry = fill_geometry(window.logical(), Some(&layout)).unwrap();
        assert_eq!(geometry.frame, frame, "{mode:?}");
        assert_eq!(geometry.left, layout.left, "{mode:?}");
        assert_eq!(geometry.right, layout.right, "{mode:?}");
    }
    // Integer-square at 1920x1080: 5x, 1600x1000, so 160-pixel bars.
    let policy = ScaleFit::new(ScalePolicy::new(ScaleMode::IntegerSquare, PixelAspect::Dos));
    let layout = solve_frame(
        window,
        image,
        &[],
        &SideSpaceSettings::default(),
        Some(&policy as &dyn FrameFit),
    )
    .layout;
    let geometry = fill_geometry(window.logical(), Some(&layout)).unwrap();
    assert!(close(geometry.left.unwrap(), rect(0.0, 0.0, 160.0, 1080.0)));
    assert!(close(
        geometry.right.unwrap(),
        rect(1760.0, 0.0, 1920.0, 1080.0)
    ));
    assert_eq!(geometry.frame_pixel(), Vec2::new(5.0, 5.0));

    // A layout that does not own the frame leaves it to the 1.0.1 fit.
    let idle = solve_side_space(window.logical(), image, &[], &SideSpaceSettings::default()).layout;
    assert_eq!(
        fill_geometry(window.logical(), Some(&idle)),
        fill_geometry(window.logical(), None)
    );
}

#[test]
fn a_ribbon_bar_is_whole_frame_pixels_against_the_frame() {
    let geometry = fill_geometry(Vec2::new(1920.0, 1080.0), None).unwrap();
    // 240 / 4.5 = 53.3 frame pixels: 53, against the frame.
    let (left, node) = RibbonBar::of(&geometry, BarSide::Left).unwrap();
    assert_eq!(left.size, UVec2::new(53, 200));
    assert_eq!(left.top, 0);
    assert!(close(node, rect(240.0 - 53.0 * 4.5, 0.0, 240.0, 1080.0)));
    let (right, node) = RibbonBar::of(&geometry, BarSide::Right).unwrap();
    assert_eq!(right.size, UVec2::new(53, 200));
    assert!(close(node, rect(1680.0, 0.0, 1680.0 + 53.0 * 4.5, 1080.0)));

    // A window taller than the frame (integer modes): the ribbon starts on
    // the frame's top row.
    let geometry = FillGeometry {
        frame: rect(240.0, 40.0, 1680.0, 1040.0),
        left: Some(rect(0.0, 0.0, 240.0, 1080.0)),
        right: Some(rect(1680.0, 0.0, 1920.0, 1080.0)),
    };
    let (bar, _) = RibbonBar::of(&geometry, BarSide::Left).unwrap();
    assert_eq!(bar.size, UVec2::new(53, 216));
    assert_eq!(bar.top, 8);

    // Too narrow for a box: no ribbon.
    let geometry = fill_geometry(Vec2::new(1500.0, 1080.0), None).unwrap();
    assert_eq!(RibbonBar::of(&geometry, BarSide::Left), None);
}

// --- The ribbon's pixels. --------------------------------------------------

/// A font with only the three corner glyphs, made up for the test: a
/// diagonal bevel whose clear bits are known.
fn corner_font() -> FixedCellFont {
    let mut bytes = vec![0x00; CH_FONT_LEN];
    let bevel: [u8; 8] = [0x1f, 0x3f, 0x7f, 0xff, 0xff, 0xff, 0xff, 0xff];
    let mut flipped = bevel;
    flipped.reverse();
    for (code, rows) in [
        (0x7bu8, bevel),
        (0x7c, bevel.map(u8::reverse_bits)),
        (0x7d, flipped),
    ] {
        bytes[usize::from(code) * 8..usize::from(code) * 8 + 8].copy_from_slice(&rows);
    }
    parse_ch_font(&bytes, IBM_CH_FILE).unwrap()
}

fn pixel(rgba: &[u8], width: u32, x: u32, y: u32) -> [u8; 3] {
    let offset = ((y * width + x) * 4) as usize;
    [rgba[offset], rgba[offset + 1], rgba[offset + 2]]
}

const BLUE: [u8; 3] = [0x00, 0x00, 0xaa];
const WHITE: [u8; 3] = [0xff, 0xff, 0xff];
const BLACK: [u8; 3] = [0x00, 0x00, 0x00];

#[test]
fn the_ribbon_is_the_border_traced_round_the_bar() {
    let bar = RibbonBar {
        side: BarSide::Left,
        size: UVec2::new(53, 200),
        top: 0,
    };
    let rgba = paint_ribbon_bar(&corner_font(), bar, ChromePalette::EGA);
    assert_eq!(rgba.len(), 53 * 200 * 4);
    let at = |x, y| pixel(&rgba, 53, x, y);
    // The gutters and the rows below the border's last row are black.
    for y in 0..200 {
        for x in (0..8).chain(45..53) {
            assert_eq!(at(x, y), BLACK, "gutter ({x},{y})");
        }
    }
    for y in 192..200 {
        for x in 0..53 {
            assert_eq!(at(x, y), BLACK, "below the border ({x},{y})");
        }
    }
    // The band is seven pixels of blue, the rule the eighth, then black.
    for y in [40, 100, 150] {
        for x in 8..15 {
            assert_eq!(at(x, y), BLUE, "left band ({x},{y})");
        }
        assert_eq!(at(15, y), WHITE, "left rule");
        assert_eq!(at(16, y), BLACK, "inside");
        assert_eq!(at(36, y), BLACK, "inside");
        assert_eq!(at(37, y), WHITE, "right rule");
        for x in 38..45 {
            assert_eq!(at(x, y), BLUE, "right band ({x},{y})");
        }
    }
    for x in [20, 26, 30] {
        for y in 0..7 {
            assert_eq!(at(x, y), BLUE, "top band ({x},{y})");
        }
        assert_eq!(at(x, 7), WHITE, "top rule");
        assert_eq!(at(x, 184), WHITE, "bottom rule");
        for y in 185..192 {
            assert_eq!(at(x, y), BLUE, "bottom band ({x},{y})");
        }
    }
    // The corners are carved by the font's glyphs, the fourth mirrored:
    // the outermost pixels are clear, the next ones in are band.
    for (x, y) in [(8, 0), (44, 0), (8, 191), (44, 191)] {
        assert_eq!(at(x, y), BLACK, "carved corner ({x},{y})");
    }
    for (x, y) in [(11, 0), (41, 0), (11, 191), (41, 191), (8, 3), (44, 188)] {
        assert_eq!(at(x, y), BLUE, "corner band ({x},{y})");
    }
    // The rule closes over the corner cells.
    for (x, y) in [(15, 7), (37, 7), (15, 184), (37, 184)] {
        assert_eq!(at(x, y), WHITE, "rule corner ({x},{y})");
    }
}

#[test]
fn the_ribbon_starts_on_the_frame_top_in_a_taller_window() {
    let bar = RibbonBar {
        side: BarSide::Right,
        size: UVec2::new(53, 216),
        top: 8,
    };
    let rgba = paint_ribbon_bar(&corner_font(), bar, ChromePalette::EGA);
    let at = |x, y| pixel(&rgba, 53, x, y);
    for y in 0..8 {
        assert_eq!(at(26, y), BLACK, "above the frame");
    }
    assert_eq!(at(26, 8), BLUE, "the frame's row 0");
    assert_eq!(at(26, 8 + 7), WHITE, "the frame's row 7");
    assert_eq!(at(26, 8 + 184), WHITE, "the frame's row 184");
    assert_eq!(at(26, 8 + 191), BLUE, "the frame's row 191");
    for y in 8 + 192..216 {
        assert_eq!(at(26, y), BLACK, "below the frame's row 191");
    }
}

// --- The bezel. ------------------------------------------------------------

#[test]
fn the_bezel_covers_the_window_and_shows_through_the_bars() {
    // A picture the window's own size maps one to one.
    let window = Vec2::new(1920.0, 1080.0);
    let size = UVec2::new(1920, 1080);
    assert!(close(
        bezel_crop(size, window, rect(0.0, 0.0, 240.0, 1080.0)),
        rect(0.0, 0.0, 240.0, 1080.0)
    ));
    // A half-size picture is scaled twice.
    assert!(close(
        bezel_crop(
            UVec2::new(960, 540),
            window,
            rect(1680.0, 0.0, 1920.0, 1080.0)
        ),
        rect(840.0, 0.0, 960.0, 540.0)
    ));
    // A square picture in a wide window covers the width and is cropped
    // top and bottom, centred.
    let crop = bezel_crop(
        UVec2::new(1000, 1000),
        Vec2::new(2000.0, 1000.0),
        rect(0.0, 0.0, 250.0, 1000.0),
    );
    assert!(close(crop, rect(0.0, 250.0, 125.0, 750.0)), "{crop:?}");
}

#[test]
fn only_a_visible_fill_draws() {
    for (fill, visible) in [
        (PillarFill::Black, false),
        (PillarFill::Solid([0, 0, 0]), false),
        (PillarFill::Solid([0xaa, 0x55, 0x00]), true),
        (PillarFill::Ribbon, true),
        (PillarFill::Bezel(None), false),
        (PillarFill::Bezel(Some(bezel(1, 1))), true),
    ] {
        assert_eq!(fill_visible(&fill), visible, "{fill:?}");
    }
}

// --- In the shell. ---------------------------------------------------------

fn resolved(cli: &[CliSetting]) -> VisualLaunchConfig {
    VisualLaunchConfig {
        enhancements: u5_runtime::enhancements::resolve(ResolveInput {
            stored: None,
            cli,
            lock: LockState::Off,
            caps: HostCapabilities::ALL,
        }),
        ..VisualLaunchConfig::classic()
    }
}

fn set(key: &str, value: &str) -> CliSetting {
    CliSetting::Set {
        key: key.to_string(),
        value: value.to_string(),
    }
}

fn spawn_window(app: &mut App, width: f32, height: f32) {
    app.world_mut().spawn(Window {
        resolution: WindowResolution::new(width, height),
        ..default()
    });
}

fn fill_shell(launch: &VisualLaunchConfig, width: f32, height: f32) -> Shell {
    let mut shell = Shell::new(
        test_state(open_grid(), 5, 5),
        &Scene::synthetic(),
        launch,
        |_| {},
    );
    spawn_window(&mut shell.app, width, height);
    shell
}

fn resize(shell: &mut Shell, width: f32, height: f32) {
    let world = shell.app.world_mut();
    let mut windows = world.query::<&mut Window>();
    windows
        .single_mut(world)
        .unwrap()
        .resolution
        .set(width, height);
}

/// The fill nodes: side, rect and what they show, left first.
fn fill_nodes(shell: &mut Shell) -> Vec<(BarSide, Rect, ImageNode)> {
    let world = shell.app.world_mut();
    let mut query = world.query::<(&PillarFillNode, &Node, &ImageNode, &GlobalZIndex)>();
    let px = |value: Val| match value {
        Val::Px(px) => px,
        other => panic!("not pixels: {other:?}"),
    };
    let mut nodes: Vec<_> = query
        .iter(world)
        .map(|(fill, node, image, z)| {
            assert_eq!(*z, GlobalZIndex(-1), "under every other node");
            assert_eq!(node.position_type, PositionType::Absolute);
            let (left, top) = (px(node.left), px(node.top));
            let rect = Rect::new(left, top, left + px(node.width), top + px(node.height));
            (fill.side, rect, image.clone())
        })
        .collect();
    nodes.sort_by_key(|(side, ..)| *side == BarSide::Right);
    nodes
}

fn fill_systems(app: &App) -> bool {
    app.world()
        .resource::<bevy::ecs::schedule::Schedules>()
        .iter()
        .any(|(_, schedule)| {
            schedule
                .graph()
                .systems()
                .any(|(_, system, _)| system.name().contains("draw_pillar_fill"))
        })
}

fn bezel(width: u32, height: u32) -> BezelImage {
    BezelImage {
        width,
        height,
        rgba: [0x10, 0x20, 0x30, 0xff]
            .repeat((width * height) as usize)
            .into(),
    }
}

#[test]
fn the_fill_rows_are_shipped_and_reach_the_settings() {
    for key in [
        "display.pillar_fill",
        "display.pillar_color",
        "display.bezel_image",
    ] {
        let row = u5_runtime::enhancements::CatalogueRow::find(key).expect(key);
        assert!(row.is_shipped(), "{key}");
    }
    // The wider view stays hidden until its package.
    for key in ["display.grow_left", "display.crawl"] {
        let row = u5_runtime::enhancements::CatalogueRow::find(key).expect(key);
        assert!(!row.is_shipped(), "{key}");
    }
    let launch = resolved(&[
        set("display.pillar_fill", "solid"),
        set("display.pillar_color", "#AA5500"),
    ]);
    assert!(launch.enhancements.notes.is_empty());
    assert_eq!(
        launch.enhancements.display().pillar_fill,
        PillarFill::Solid([0xaa, 0x55, 0x00])
    );
    // The colour alone is a sub-option of an off fill: still Classic.
    assert!(!resolved(&[set("display.pillar_color", "#AA5500")]).wants_enhancements());
}

#[test]
fn a_black_fill_has_no_node_and_no_system() {
    // `black` with another display key on, and `solid` black.
    for (cli, chosen) in [
        (vec![set("display.scale_mode", "sharp")], false),
        (vec![set("display.pillar_fill", "solid")], true),
        (
            vec![
                set("display.pillar_fill", "solid"),
                set("display.pillar_color", "#000000"),
            ],
            true,
        ),
    ] {
        let launch = resolved(&cli);
        let mut shell = fill_shell(&launch, 1920.0, 1080.0);
        assert!(shell.app.is_plugin_added::<DisplayPlugin>(), "{cli:?}");
        // The schedules list their systems until their first run.
        assert_eq!(fill_systems(&shell.app), chosen, "{cli:?}");
        for _ in 0..3 {
            shell.step(&[]);
        }
        assert_eq!(fill_nodes(&mut shell).len(), 0, "{cli:?}");
    }
}

#[test]
fn a_solid_fill_covers_both_bars() {
    let launch = resolved(&[
        set("display.pillar_fill", "solid"),
        set("display.pillar_color", "#AA5500"),
    ]);
    let mut shell = fill_shell(&launch, 1920.0, 1080.0);
    shell.step(&[]);
    shell.step(&[]);
    let nodes = fill_nodes(&mut shell);
    assert_eq!(nodes.len(), 2);
    let brown = Color::srgb_u8(0xaa, 0x55, 0x00);
    let expected = [
        (BarSide::Left, rect(0.0, 0.0, 240.0, 1080.0)),
        (BarSide::Right, rect(1680.0, 0.0, 1920.0, 1080.0)),
    ];
    for ((side, node, image), (want_side, want_rect)) in nodes.iter().zip(expected) {
        assert_eq!(*side, want_side);
        assert!(close(*node, want_rect), "{side:?}: {node:?}");
        assert_eq!(image.color, brown);
    }

    // A resize moves them; a 4:3 window has none.
    resize(&mut shell, 3440.0, 1440.0);
    shell.step(&[]);
    let nodes = fill_nodes(&mut shell);
    assert!(close(nodes[0].1, rect(0.0, 0.0, 760.0, 1440.0)));
    assert!(close(nodes[1].1, rect(2680.0, 0.0, 3440.0, 1440.0)));
    resize(&mut shell, 1600.0, 1200.0);
    shell.step(&[]);
    assert_eq!(fill_nodes(&mut shell).len(), 0);
}

#[test]
fn the_ribbon_paints_non_black_bars() {
    let launch = resolved(&[set("display.pillar_fill", "ribbon")]);
    let mut shell = fill_shell(&launch, 1920.0, 1080.0);
    shell.step(&[]);
    shell.step(&[]);
    let nodes = fill_nodes(&mut shell);
    assert_eq!(nodes.len(), 2);
    let images = shell.app.world().resource::<Assets<Image>>();
    for (side, node, image) in &nodes {
        assert_eq!(node.height(), 1080.0, "{side:?}");
        assert!((node.width() - 53.0 * 4.5).abs() < 1.0e-3, "{side:?}");
        let pixels = images.get(&image.image).expect("the ribbon image");
        assert_eq!(pixels.size(), UVec2::new(53, 200));
        let data = pixels.data.as_ref().unwrap();
        // Checked only for colour, with no art hash: the blue band and the
        // white rule are there.
        let colours: std::collections::BTreeSet<[u8; 3]> = data
            .as_chunks::<4>()
            .0
            .iter()
            .map(|pixel| [pixel[0], pixel[1], pixel[2]])
            .collect();
        assert!(colours.contains(&BLUE), "{side:?}");
        assert!(colours.contains(&WHITE), "{side:?}");
    }
}

#[test]
fn a_bezel_shows_through_the_bars_and_falls_back_to_black() {
    // The launch's decode.
    let mut launch = resolved(&[set("display.pillar_fill", "bezel")]);
    assert_eq!(
        launch.enhancements.display().pillar_fill,
        PillarFill::Bezel(None)
    );
    launch.bezel = Some(bezel(960, 540));
    let mut shell = fill_shell(&launch, 1920.0, 1080.0);
    shell.step(&[]);
    let nodes = fill_nodes(&mut shell);
    assert_eq!(nodes.len(), 2);
    assert!(close(
        nodes[0].2.rect.unwrap(),
        rect(0.0, 0.0, 120.0, 540.0)
    ));
    assert!(close(
        nodes[1].2.rect.unwrap(),
        rect(840.0, 0.0, 960.0, 540.0)
    ));
    let images = shell.app.world().resource::<Assets<Image>>();
    let shown = images.get(&nodes[0].2.image).unwrap();
    assert_eq!(shown.size(), UVec2::new(960, 540));

    // The host could not decode it: black, no node.
    let launch = resolved(&[set("display.pillar_fill", "bezel")]);
    let mut shell = fill_shell(&launch, 1920.0, 1080.0);
    shell.step(&[]);
    assert_eq!(fill_nodes(&mut shell).len(), 0);
}

#[test]
fn a_bezel_chosen_in_play_arrives_through_the_mailbox() {
    let mailbox = BezelMailbox::new();
    let launch = VisualLaunchConfig {
        bezel_updates: Some(mailbox.clone()),
        ..resolved(&[set("display.pillar_fill", "bezel")])
    };
    let mut shell = fill_shell(&launch, 1920.0, 1080.0);
    shell.step(&[]);
    assert_eq!(fill_nodes(&mut shell).len(), 0);

    mailbox.post(BezelDecode::Image(bezel(1920, 1080)));
    shell.step(&[]);
    let nodes = fill_nodes(&mut shell);
    assert_eq!(nodes.len(), 2);
    assert!(close(
        nodes[1].2.rect.unwrap(),
        rect(1680.0, 0.0, 1920.0, 1080.0)
    ));

    // A file that could not be used: black again, with a note.
    mailbox.post(BezelDecode::Failed("not a PNG".into()));
    shell.step(&[]);
    assert_eq!(fill_nodes(&mut shell).len(), 0);
    let toasts = shell.app.world().resource::<Toasts>();
    assert_eq!(toasts.current(), Some(BEZEL_REFUSED));
}

#[test]
fn changing_the_fill_in_play_replaces_or_removes_its_nodes() {
    let launch = resolved(&[set("display.pillar_fill", "ribbon")]);
    let mut shell = fill_shell(&launch, 1920.0, 1080.0);
    shell.step(&[]);
    assert_eq!(fill_nodes(&mut shell).len(), 2);
    shell
        .app
        .world_mut()
        .resource_mut::<Display>()
        .0
        .pillar_fill = PillarFill::Solid([0, 0, 0x55]);
    shell.step(&[]);
    let nodes = fill_nodes(&mut shell);
    assert_eq!(nodes.len(), 2);
    assert_eq!(nodes[0].2.color, Color::srgb_u8(0, 0, 0x55));
    shell
        .app
        .world_mut()
        .resource_mut::<Display>()
        .0
        .pillar_fill = PillarFill::Black;
    shell.step(&[]);
    assert_eq!(fill_nodes(&mut shell).len(), 0);
    // Nothing left to remove: the system stops running.
    shell.step(&[]);
    assert_eq!(fill_nodes(&mut shell).len(), 0);
}

#[test]
fn the_fill_follows_the_scale_policy() {
    let launch = resolved(&[
        set("display.pillar_fill", "solid"),
        set("display.pillar_color", "#AA5500"),
        set("display.scale_mode", "integer-square"),
    ]);
    let mut shell = fill_shell(&launch, 1920.0, 1080.0);
    shell.step(&[]);
    shell.step(&[]);
    let nodes = fill_nodes(&mut shell);
    assert!(
        close(nodes[0].1, rect(0.0, 0.0, 160.0, 1080.0)),
        "{nodes:?}"
    );
    assert!(close(nodes[1].1, rect(1760.0, 0.0, 1920.0, 1080.0)));
}

#[test]
fn every_fill_leaves_the_game_and_its_frame_alone() {
    let start = test_state(open_grid(), 5, 5);
    let mut timeline = Timeline::new(70);
    timeline
        .game_tap(5, KeyCode::ArrowUp)
        .game_tap(20, KeyCode::ArrowLeft)
        .game_tap(35, KeyCode::ArrowDown)
        .game_tap(50, KeyCode::ArrowRight);
    for (cli, with_bezel) in [
        (vec![set("display.pillar_fill", "ribbon")], false),
        (
            vec![
                set("display.pillar_fill", "solid"),
                set("display.pillar_color", "#AA5500"),
            ],
            false,
        ),
        (vec![set("display.pillar_fill", "bezel")], true),
    ] {
        let mut launch = resolved(&cli);
        if with_bezel {
            launch.bezel = Some(bezel(64, 36));
        }
        let mismatches =
            check_shell_equivalence(&start, &Scene::synthetic(), &timeline, &launch, |app| {
                spawn_window(app, 1920.0, 1080.0)
            });
        assert_eq!(mismatches, [], "{cli:?}");
    }
}

#[test]
fn under_sharp_the_ribbon_is_prescaled_like_the_frame() {
    let launch = resolved(&[
        set("display.pillar_fill", "ribbon"),
        set("display.scale_mode", "sharp"),
    ]);
    let mut shell = fill_shell(&launch, 2560.0, 1080.0);
    shell.step(&[]);
    shell.step(&[]);
    let nodes = fill_nodes(&mut shell);
    assert_eq!(nodes.len(), 2);
    let images = shell.app.world().resource::<Assets<Image>>();
    for (side, node, image) in &nodes {
        // Sharp at 2560x1080: the frame is 1440x1080, 4.5 x 5.4 per frame
        // pixel, so the bars are 560 wide: 124 frame pixels, expanded 4x5.
        assert!((node.width() - 124.0 * 4.5).abs() < 1.0e-3, "{side:?}");
        let shown = images.get(&image.image).unwrap();
        assert_eq!(shown.size(), UVec2::new(124 * 4, 200 * 5), "{side:?}");
        assert_eq!(
            format!("{:?}", shown.sampler),
            format!("{:?}", ImageSampler::linear()),
            "{side:?}"
        );
    }
    // Under fit it is the plain frame-pixel image, sampled nearest.
    let launch = resolved(&[set("display.pillar_fill", "ribbon")]);
    let mut shell = fill_shell(&launch, 2560.0, 1080.0);
    shell.step(&[]);
    let nodes = fill_nodes(&mut shell);
    let images = shell.app.world().resource::<Assets<Image>>();
    let shown = images.get(&nodes[0].2.image).unwrap();
    assert_eq!(shown.size(), UVec2::new(124, 200));
    assert_eq!(
        format!("{:?}", shown.sampler),
        format!("{:?}", ImageSampler::nearest())
    );
}

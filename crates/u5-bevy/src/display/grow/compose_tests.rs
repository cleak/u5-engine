//! The grow-left compositor, steps 1-4 (design 2.3.5, 2.3.7; build plan
//! M1-09): the peek window of each canvas, the chrome, the ring placement,
//! crop-back identity, the flash on the ring only, and in the shell: the
//! canvas in place of the frame, the frame fit sizing it, fixed screens
//! passing through, every upload reaching the core, a new presented image
//! on resize, and the game left alone.

use bevy::window::WindowResolution;
use u5_runtime::enhancements::{
    CliSetting, DISPLAY_P1, DISPLAY_P2, DISPLAY_P8, HostCapabilities, LockState, ResolveInput,
    ScaleMode, resolve_as_shipped,
};
use u5_runtime::test_fixtures::{open_grid, test_state};
use u5_runtime::{PeekWindow, potion_flash_playback};

use super::compose::{
    BAND_BLUE, RULE_WHITE, compose_grow_left, core_rect, paint_chrome, peek_window,
};
use super::*;
use crate::EGA_PALETTE_RGB;
use crate::display::scale::{ScaleFit, ScalePolicy};
use crate::enh::VisualLaunchConfig;
use crate::enh::equivalence::{Scene, Shell, Timeline, check_shell_equivalence};
use crate::enh::present::PresentHost;
use crate::enh::side::{FrameFit, WindowPixels};

fn canvas(w: u32, h: u32) -> Canvas {
    Canvas::for_window(w, h, PixelAspect::Dos).unwrap()
}

fn rgb(index: u8) -> [u8; 3] {
    EGA_PALETTE_RGB[usize::from(index)]
}

fn at(rgba: &[u8], width: u32, x: u32, y: u32) -> [u8; 3] {
    let i = ((y * width + x) * 4) as usize;
    [rgba[i], rgba[i + 1], rgba[i + 2]]
}

/// A 320x200 frame of EGA colours, distinct from pixel to pixel.
fn faithful_pattern(seed: usize) -> Vec<u8> {
    (0..(320 * 200))
        .flat_map(|i: usize| {
            let [r, g, b] = EGA_PALETTE_RGB[(i * 7 + i / 320 + seed) % 16];
            [r, g, b, 0xff]
        })
        .collect()
}

/// A ring raster whose every cell is one EGA colour, picked by its place.
fn ring_pattern(window: PeekWindow) -> Vec<u8> {
    let (w, h) = (window.columns() * 16, window.rows() * 16);
    (0..w * h)
        .flat_map(|i| {
            let (cell_x, cell_y) = ((i % w) / 16, (i / w) / 16);
            let [r, g, b] = EGA_PALETTE_RGB[(cell_x * 3 + cell_y * 5) % 16];
            [r, g, b, 0xff]
        })
        .collect()
}

// --- Pure compose. ---------------------------------------------------------

#[test]
fn each_canvas_asks_the_peek_for_exactly_its_map_window() {
    for ((ww, wh), window) in [
        ((2560, 1440), (15, 15, 7, 7)),
        ((3440, 1440), (21, 21, 7, 7)),
        ((1920, 1080), (14, 14, 6, 6)),
        ((2560, 1080), (19, 19, 6, 6)),
    ] {
        let c = canvas(ww, wh);
        let p = peek_window(&c);
        assert_eq!((p.west, p.east, p.north, p.south), window, "{ww}x{wh}");
        assert!(p.is_valid());
    }
    // A panned canvas reaches further west than east, and every cell of
    // its map window is covered.
    for (ww, wh) in [(1920, 1200), (1600, 1200), (1024, 768), (800, 600)] {
        let c = canvas(ww, wh);
        let p = peek_window(&c);
        let (x0, y0, x1, y1) = c.map_window();
        assert!(c.party_x - u32::from(p.west) * 16 <= x0, "{ww}x{wh}");
        assert!(c.party_x + 16 + u32::from(p.east) * 16 > x1, "{ww}x{wh}");
        assert!(c.party_y - u32::from(p.north) * 16 <= y0, "{ww}x{wh}");
        assert!(c.party_y + 16 + u32::from(p.south) * 16 > y1, "{ww}x{wh}");
        if c.pan_tiles > 0 {
            assert!(p.west > p.east, "{ww}x{wh}: {p:?}");
        }
    }
}

#[test]
fn the_chrome_is_the_band_the_rule_and_the_rounded_corners() {
    for (ww, wh) in [(2560, 1440), (3440, 1440), (1920, 1080)] {
        let c = canvas(ww, wh);
        let (w, h) = (c.w, c.h);
        let mut rgba = vec![0; (w * h * 4) as usize];
        paint_chrome(&mut rgba, &c);
        let blue = rgb(BAND_BLUE);
        let white = rgb(RULE_WHITE);
        let black = rgb(0);
        // The rule: x 7 and W - 8, y 15 and H - 8, round a black window.
        for y in [15, 100, h - 8] {
            assert_eq!(at(&rgba, w, 7, y), white, "{w}: left rule y {y}");
            assert_eq!(at(&rgba, w, w - 8, y), white, "{w}: right rule y {y}");
            assert_eq!(at(&rgba, w, 6, y), blue, "{w}");
            assert_eq!(at(&rgba, w, w - 7, y), blue, "{w}");
        }
        for x in [7, 200, w - 8] {
            assert_eq!(at(&rgba, w, x, 15), white, "{w}: top rule x {x}");
            assert_eq!(at(&rgba, w, x, h - 8), white, "{w}: bottom rule x {x}");
            assert_eq!(at(&rgba, w, x, 14), blue, "{w}");
            assert_eq!(at(&rgba, w, x, h - 7), blue, "{w}");
        }
        assert_eq!(at(&rgba, w, 8, 16), black);
        assert_eq!(at(&rgba, w, w - 9, h - 9), black);
        // 7 blue columns each side, 15 blue rows on top, 7 below.
        for x in 0..7 {
            assert_eq!(at(&rgba, w, x, 100), blue);
            assert_eq!(at(&rgba, w, w - 1 - x, 100), blue);
        }
        // The corners, at all four: row r in from an edge starts at
        // column CHROME_CORNER_PROFILE[r], black before it.
        for (r, first) in u5_runtime::CHROME_CORNER_PROFILE.iter().enumerate() {
            let first = u32::from(*first);
            let r = r as u32;
            for y in [r, h - 1 - r] {
                for x in 0..7 {
                    let want = if x < first { black } else { blue };
                    assert_eq!(at(&rgba, w, x, y), want, "{w}: ({x}, {y})");
                    assert_eq!(at(&rgba, w, w - 1 - x, y), want, "{w}: far ({x}, {y})");
                }
            }
        }
    }
}

#[test]
fn the_ring_sits_on_the_party_tile_and_the_core_is_the_faithful_viewport() {
    for (ww, wh) in [(2560, 1440), (3440, 1440), (1920, 1080), (1920, 1200)] {
        let c = canvas(ww, wh);
        let window = peek_window(&c);
        let ring = ring_pattern(window);
        let faithful = faithful_pattern(ww as usize);
        let rgba = compose_grow_left(&faithful, &ring, window, &c, None, None, None);
        assert_eq!(rgba.len(), (c.w * c.h * 4) as usize);
        let ring_w = (window.columns() * 16) as u32;
        let (ox, oy) = (
            c.party_x - u32::from(window.west) * 16,
            c.party_y - u32::from(window.north) * 16,
        );
        let (cx, cy, side) = core_rect(&c);
        let (mx0, my0, mx1, my1) = c.map_window();
        for y in my0..=my1 {
            for x in mx0..=mx1 {
                let core = (cx..cx + side).contains(&(x as usize))
                    && (cy..cy + side).contains(&(y as usize));
                let want = if core {
                    at(&faithful, 320, 8 + x - cx as u32, 8 + y - cy as u32)
                } else {
                    at(&ring, ring_w, x - ox, y - oy)
                };
                assert_eq!(at(&rgba, c.w, x, y), want, "{ww}x{wh} ({x}, {y})");
            }
        }
        // The core is centred on the party tile: the party cell of the
        // faithful viewport (x and y 88..103) lands on it.
        assert_eq!(
            at(&rgba, c.w, c.party_x, c.party_y),
            at(&faithful, 320, 88, 88)
        );
    }
}

#[test]
fn the_flash_inverts_the_ring_and_leaves_the_core_to_the_frame() {
    let playback = potion_flash_playback(0).unwrap();
    // The frame's own flash covers exactly the viewport the core copies.
    assert_eq!(
        (
            playback.playfield_left,
            playback.playfield_top,
            playback.playfield_right,
            playback.playfield_bottom
        ),
        (8, 8, 183, 183)
    );
    let c = canvas(2560, 1440);
    let window = peek_window(&c);
    let ring = ring_pattern(window);
    let faithful = faithful_pattern(3);
    let plain = compose_grow_left(&faithful, &ring, window, &c, None, None, None);
    let flashed = compose_grow_left(&faithful, &ring, window, &c, Some(playback), None, None);
    let (cx, cy, side) = core_rect(&c);
    let (mx0, my0, mx1, my1) = c.map_window();
    let index = |px: [u8; 3]| EGA_PALETTE_RGB.iter().position(|e| *e == px).unwrap() as u8;
    for y in 0..c.h {
        for x in 0..c.w {
            let core =
                (cx..cx + side).contains(&(x as usize)) && (cy..cy + side).contains(&(y as usize));
            let map = (mx0..=mx1).contains(&x) && (my0..=my1).contains(&y);
            let (p, f) = (at(&plain, c.w, x, y), at(&flashed, c.w, x, y));
            if map && !core {
                assert_eq!(index(p) ^ playback.palette_xor_mask, index(f), "({x}, {y})");
            } else {
                // The core (already inverted in the frame) and the chrome.
                assert_eq!(p, f, "({x}, {y})");
            }
        }
    }
}

#[test]
fn no_state_or_a_fixed_screen_gives_no_canvas() {
    let window = UVec2::new(2560, 1440);
    assert_eq!(grow_canvas(None, window, PixelAspect::Dos), None);
    let mut state = test_state(open_grid(), 5, 5);
    assert_eq!(
        grow_canvas(Some(&state), window, PixelAspect::Dos),
        Some(canvas(2560, 1440))
    );
    assert_eq!(
        grow_canvas(Some(&state), UVec2::new(300, 200), PixelAspect::Dos),
        None
    );
    state.cutscene_black_viewport = true;
    assert_eq!(grow_canvas(Some(&state), window, PixelAspect::Dos), None);
}

// --- The frame fit. --------------------------------------------------------

#[test]
fn the_frame_fit_places_the_canvas_under_every_mode() {
    let fit = |mode: ScaleMode, ww: u32, wh: u32| {
        let c = canvas(ww, wh);
        let window = WindowPixels {
            physical: UVec2::new(ww, wh),
            scale_factor: 1.0,
        };
        ScaleFit::new(ScalePolicy::new(mode, PixelAspect::Dos))
            .with_canvas(true)
            .place(window, UVec2::new(c.w, c.h))
            .unwrap()
    };
    // The named layouts fill the window exactly, under Fit and Sharp and
    // the hybrid integer mode alike.
    for (ww, wh) in [(2560, 1440), (3440, 1440), (1920, 1080)] {
        for mode in [ScaleMode::Fit, ScaleMode::Sharp, ScaleMode::IntegerHybrid] {
            let rect = fit(mode, ww, wh);
            assert!(
                (rect.min - Vec2::ZERO).length() < 1.0e-3
                    && (rect.max - Vec2::new(ww as f32, wh as f32)).length() < 1.0e-3,
                "{mode:?} {ww}x{wh}: {rect:?}"
            );
        }
    }
    // Square integers ignore the aspect: 5x5 over 512x240 at 2560x1440.
    let rect = fit(ScaleMode::IntegerSquare, 2560, 1440);
    assert_eq!(rect.size(), Vec2::new(2560.0, 1200.0));
    // The 320x200 frame is still the faithful fit's under Fit.
    let window = WindowPixels {
        physical: UVec2::new(2560, 1440),
        scale_factor: 1.0,
    };
    let policy = ScaleFit::new(ScalePolicy::default()).with_canvas(true);
    assert_eq!(policy.place(window, UVec2::new(320, 200)), None);
    // Without the canvas flag a canvas-sized image is left to the old path.
    let plain = ScaleFit::new(ScalePolicy::default());
    assert_eq!(plain.place(window, UVec2::new(512, 240)), None);
}

// --- In the shell. ---------------------------------------------------------

fn launch(cli: &[CliSetting]) -> VisualLaunchConfig {
    VisualLaunchConfig {
        enhancements: resolve_as_shipped(
            ResolveInput {
                stored: None,
                cli,
                lock: LockState::Off,
                caps: HostCapabilities::ALL,
            },
            &[DISPLAY_P1, DISPLAY_P2, DISPLAY_P8],
        ),
        ..VisualLaunchConfig::classic()
    }
}

fn grow_on() -> VisualLaunchConfig {
    launch(&[CliSetting::Enable("display.grow_left".into())])
}

fn spawn_window(app: &mut App, width: f32, height: f32) {
    app.world_mut().spawn(Window {
        resolution: WindowResolution::new(width, height),
        ..default()
    });
}

fn grow_shell(launch: &VisualLaunchConfig, width: f32, height: f32) -> Shell {
    grow_shell_with(launch, width, height, |_| {})
}

fn grow_shell_with(
    launch: &VisualLaunchConfig,
    width: f32,
    height: f32,
    install: impl FnOnce(&mut App),
) -> Shell {
    let mut shell = Shell::new(
        test_state(open_grid(), 5, 5),
        &Scene::synthetic(),
        launch,
        install,
    );
    spawn_window(&mut shell.app, width, height);
    shell
}

fn close(size: Option<Vec2>, want: Vec2) -> bool {
    size.is_some_and(|size| (size - want).length() < 1.0e-2)
}

/// A test-only upload site: bytes written into the faithful image after
/// every faithful system of the frame and before the pipeline, as the
/// latest upload site (the camp flash) writes them.
#[derive(Resource, Default)]
struct LateUpload {
    bytes: Option<Vec<u8>>,
    flash: bool,
}

fn late_upload(
    mut upload: ResMut<LateUpload>,
    mut visual: ResMut<VisualState>,
    mut images: ResMut<Assets<Image>>,
) {
    if upload.flash {
        visual.state.presentation_camp_flash_inverted = true;
    }
    if let Some(bytes) = upload.bytes.take() {
        images.get_mut(&visual.image_handle).unwrap().data = Some(bytes);
    }
}

fn install_late_upload(app: &mut App) {
    app.init_resource::<LateUpload>().add_systems(
        Update,
        late_upload
            .after(crate::present_camp_apparition_flash)
            .before(EnhSet::Present),
    );
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

/// The frame sprite's image, size and the image's pixels.
fn shown(shell: &mut Shell) -> (Handle<Image>, Option<Vec2>, UVec2, Vec<u8>) {
    let world = shell.app.world_mut();
    let mut sprites = world.query::<&Sprite>();
    let sprite = sprites.single(world).unwrap();
    let (handle, size) = (sprite.image.clone(), sprite.custom_size);
    let image = world.resource::<Assets<Image>>().get(&handle).unwrap();
    (
        handle,
        size,
        image.size(),
        image.data.clone().unwrap_or_default(),
    )
}

fn faithful(shell: &Shell) -> Vec<u8> {
    let handle = shell.visual().image_handle.clone();
    shell
        .app
        .world()
        .resource::<Assets<Image>>()
        .get(&handle)
        .and_then(|image| image.data.clone())
        .unwrap()
}

/// The canvas core is the faithful viewport, byte for byte.
fn assert_core_is_faithful(canvas_rgba: &[u8], c: &Canvas, faithful: &[u8], context: &str) {
    let (cx, cy, side) = core_rect(c);
    let w = c.w as usize;
    for row in 0..side {
        let at = ((cy + row) * w + cx) * 4;
        let from = ((8 + row) * 320 + 8) * 4;
        assert_eq!(
            canvas_rgba[at..at + side * 4],
            faithful[from..from + side * 4],
            "{context}: core row {row}"
        );
    }
}

fn set_state(shell: &mut Shell, edit: impl FnOnce(&mut PlayState)) {
    edit(&mut shell.app.world_mut().resource_mut::<VisualState>().state);
}

#[test]
fn with_grow_left_off_the_frame_is_the_faithful_handle() {
    for cli in [
        vec![],
        vec![CliSetting::Set {
            key: "display.pillar_fill".into(),
            value: "ribbon".into(),
        }],
        vec![CliSetting::Set {
            key: "display.scale_mode".into(),
            value: "integer-hybrid".into(),
        }],
    ] {
        let launch = launch(&cli);
        assert!(!launch.enhancements.display().grow_left.on);
        let mut shell = grow_shell(&launch, 2560.0, 1440.0);
        for _ in 0..3 {
            shell.step(&[]);
        }
        let faithful_handle = shell.visual().image_handle.clone();
        let (handle, _, size, _) = shown(&mut shell);
        assert_eq!(handle, faithful_handle, "{cli:?}");
        assert_eq!(size, UVec2::new(320, 200), "{cli:?}");
        let world = shell.app.world();
        assert!(!world.contains_resource::<FrameImageSize>(), "{cli:?}");
        assert!(
            world
                .get_resource::<PresentSteps>()
                .is_none_or(|steps| !steps.orders().contains(&StepOrder::GROW_LEFT_COMPOSE)),
            "{cli:?}"
        );
    }
}

#[test]
fn the_canvas_replaces_the_frame_and_the_fit_sizes_it() {
    for ((ww, wh), (cw, ch)) in [
        ((2560.0, 1440.0), (512, 240)),
        ((3440.0, 1440.0), (688, 240)),
        ((1920.0, 1080.0), (480, 216)),
    ] {
        let mut shell = grow_shell(&grow_on(), ww, wh);
        shell.step(&[]);
        shell.step(&[]);
        let faithful_handle = shell.visual().image_handle.clone();
        let (handle, sprite_size, size, rgba) = shown(&mut shell);
        assert_ne!(handle, faithful_handle, "{ww}x{wh}");
        assert_eq!(size, UVec2::new(cw, ch), "{ww}x{wh}");
        // The named layouts fill the window.
        assert!(
            close(sprite_size, Vec2::new(ww, wh)),
            "{ww}x{wh}: {sprite_size:?}"
        );
        assert_eq!(
            shell.app.world().resource::<FrameImageSize>().0,
            Some(UVec2::new(cw, ch))
        );
        let c = canvas(ww as u32, wh as u32);
        assert_core_is_faithful(&rgba, &c, &faithful(&shell), "named layout");
        // The faithful image is still the 320x200 frame.
        let world = shell.app.world();
        let image = world
            .resource::<Assets<Image>>()
            .get(&faithful_handle)
            .unwrap();
        assert_eq!(image.size(), UVec2::new(320, 200));
    }
}

#[test]
fn fixed_screens_pass_the_faithful_frame_through_and_back() {
    let mut shell = grow_shell(&grow_on(), 2560.0, 1440.0);
    shell.step(&[]);
    shell.step(&[]);
    assert_eq!(shown(&mut shell).2, UVec2::new(512, 240));

    set_state(&mut shell, |state| state.cutscene_black_viewport = true);
    shell.step(&[]);
    let (_, sprite_size, size, rgba) = shown(&mut shell);
    assert_eq!(size, UVec2::new(320, 200));
    assert_eq!(rgba, faithful(&shell), "the faithful frame, untouched");
    // The 1.0.1 4:3 fit of a 2560x1440 window.
    assert!(
        close(sprite_size, Vec2::new(1920.0, 1440.0)),
        "{sprite_size:?}"
    );
    assert_eq!(shell.app.world().resource::<FrameImageSize>().0, None);

    set_state(&mut shell, |state| state.cutscene_black_viewport = false);
    shell.step(&[]);
    let (_, sprite_size, size, rgba) = shown(&mut shell);
    assert_eq!(size, UVec2::new(512, 240));
    assert!(
        close(sprite_size, Vec2::new(2560.0, 1440.0)),
        "{sprite_size:?}"
    );
    assert_core_is_faithful(&rgba, &canvas(2560, 1440), &faithful(&shell), "back");
}

#[test]
fn every_upload_reaches_the_core() {
    let mut shell = grow_shell_with(&grow_on(), 2560.0, 1440.0, install_late_upload);
    let c = canvas(2560, 1440);
    // The play input upload site: a walk.
    let mut timeline = Timeline::new(60);
    timeline
        .game_tap(5, KeyCode::ArrowUp)
        .game_tap(20, KeyCode::ArrowLeft)
        .game_tap(35, KeyCode::ArrowDown)
        .game_tap(50, KeyCode::ArrowRight);
    let mut frames = std::collections::HashSet::new();
    for frame in 0..timeline.len() {
        shell.step(timeline.frame(frame));
        let (_, _, size, rgba) = shown(&mut shell);
        assert_eq!(size, UVec2::new(512, 240));
        let faithful = faithful(&shell);
        assert_core_is_faithful(&rgba, &c, &faithful, &format!("walk frame {frame}"));
        frames.insert(faithful);
    }
    assert!(frames.len() > 1, "the walk must change the frame");
    // Any other upload site: bytes written straight into the faithful
    // image are picked up downstream.
    for seed in 0..4 {
        let bytes = faithful_pattern(seed);
        shell.app.world_mut().resource_mut::<LateUpload>().bytes = Some(bytes.clone());
        shell.app.update();
        assert_eq!(faithful(&shell), bytes);
        let (_, _, _, rgba) = shown(&mut shell);
        assert_core_is_faithful(&rgba, &c, &bytes, &format!("upload {seed}"));
    }
}

#[test]
fn the_camp_flash_inverts_the_ring_and_the_core_once() {
    let mut shell = grow_shell_with(&grow_on(), 2560.0, 1440.0, install_late_upload);
    shell.step(&[]);
    shell.step(&[]);
    let (_, _, _, calm) = shown(&mut shell);
    shell.app.world_mut().resource_mut::<LateUpload>().flash = true;
    let composes = shell.app.world().resource::<PresentHost>().composes();
    shell.app.update();
    assert!(shell.app.world().resource::<PresentHost>().composes() > composes);
    let (_, _, _, lit) = shown(&mut shell);
    let c = canvas(2560, 1440);
    // The core is whatever the frame holds; the ring is the calm ring,
    // inverted.
    assert_core_is_faithful(&lit, &c, &faithful(&shell), "flash");
    let mask = potion_flash_playback(0).unwrap().palette_xor_mask;
    let index = |px: [u8; 3]| EGA_PALETTE_RGB.iter().position(|e| *e == px).unwrap() as u8;
    for (x, y) in [
        (8, 16),
        (100, 30),
        (500, 230),
        (167, 100),
        (250, 20),
        (400, 200),
    ] {
        assert_eq!(
            index(at(&calm, 512, x, y)) ^ mask,
            index(at(&lit, 512, x, y)),
            "({x}, {y})"
        );
    }
}

#[test]
fn a_resize_makes_a_new_presented_image() {
    let mut shell = grow_shell(&grow_on(), 2560.0, 1440.0);
    shell.step(&[]);
    shell.step(&[]);
    let (first, _, size, _) = shown(&mut shell);
    assert_eq!(size, UVec2::new(512, 240));
    resize(&mut shell, 3440.0, 1440.0);
    shell.step(&[]);
    let (second, sprite_size, size, rgba) = shown(&mut shell);
    assert_ne!(first, second);
    assert_eq!(size, UVec2::new(688, 240));
    assert!(
        close(sprite_size, Vec2::new(3440.0, 1440.0)),
        "{sprite_size:?}"
    );
    assert!(
        shell
            .app
            .world()
            .resource::<Assets<Image>>()
            .get(&first)
            .is_none(),
        "the old presented image is dropped"
    );
    assert_core_is_faithful(&rgba, &canvas(3440, 1440), &faithful(&shell), "resized");
    // Too small a window: the faithful frame again.
    resize(&mut shell, 1000.0, 1000.0);
    shell.step(&[]);
    let (_, _, size, _) = shown(&mut shell);
    assert_eq!(size, UVec2::new(320, 200));
}

#[test]
fn under_sharp_the_canvas_is_prescaled() {
    let launch = launch(&[
        CliSetting::Enable("display.grow_left".into()),
        CliSetting::Set {
            key: "display.scale_mode".into(),
            value: "sharp".into(),
        },
    ]);
    let mut shell = grow_shell(&launch, 2560.0, 1440.0);
    shell.step(&[]);
    shell.step(&[]);
    let (_, sprite_size, size, _) = shown(&mut shell);
    assert_eq!(size, UVec2::new(512 * 5, 240 * 6));
    assert!(
        close(sprite_size, Vec2::new(2560.0, 1440.0)),
        "{sprite_size:?}"
    );
}

#[test]
fn grow_left_on_leaves_the_game_alone() {
    let start = test_state(open_grid(), 5, 5);
    let mut timeline = Timeline::new(70);
    timeline
        .game_tap(5, KeyCode::ArrowUp)
        .game_tap(20, KeyCode::ArrowLeft)
        .game_tap(35, KeyCode::ArrowDown)
        .game_tap(50, KeyCode::ArrowRight);
    for (ww, wh) in [(2560.0, 1440.0), (1920.0, 1080.0), (1920.0, 1200.0)] {
        let mismatches =
            check_shell_equivalence(&start, &Scene::synthetic(), &timeline, &grow_on(), |app| {
                spawn_window(app, ww, wh)
            });
        assert_eq!(mismatches, [], "{ww}x{wh}");
    }
}

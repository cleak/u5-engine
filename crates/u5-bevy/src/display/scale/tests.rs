//! The scale policies (design 2.2.11, 2.3.5; build plan M1-02): the size
//! tables for every mode and both pixel aspects on the named windows, at
//! display scales 1.0, 1.25 and 1.5, the fallbacks, and the frame fit in
//! the app.

use bevy::window::WindowResolution;
use u5_runtime::enhancements::{
    CliSetting, DISPLAY_P1, DISPLAY_P2, DISPLAY_P8, HostCapabilities, LockState, ResolveInput,
    resolve_as_shipped,
};
use u5_runtime::test_fixtures::{open_grid, test_state};

use super::*;
use crate::enh::VisualLaunchConfig;
use crate::enh::equivalence::{Scene, Shell};
use crate::enh::side::{
    PANEL_WIDTH, PanelClaim, PanelKind, SidePreference, solve_frame, solve_side_space,
};
use crate::{VISUAL_PLAY_FRAME_HEIGHT, VISUAL_PLAY_FRAME_WIDTH, fitted_display_size};

const FRAME: UVec2 = UVec2::new(VISUAL_PLAY_FRAME_WIDTH, VISUAL_PLAY_FRAME_HEIGHT);
const SCALES: [f32; 3] = [1.0, 1.25, 1.5];
const MODES: [ScaleMode; 5] = [
    ScaleMode::Fit,
    ScaleMode::Sharp,
    ScaleMode::IntegerSquare,
    ScaleMode::IntegerHybrid,
    ScaleMode::Stretch,
];
const ASPECTS: [PixelAspect; 2] = [PixelAspect::Dos, PixelAspect::Square];

/// One row: the window in physical pixels, then for the DOS and the square
/// aspect the frame's physical size and whole factor under each mode in
/// [`MODES`] order. Fit is unrounded; the table gives its whole-pixel floor.
type Row = (u32, u32, [[(u32, u32, Option<u32>); 5]; 2]);

#[rustfmt::skip]
const TABLE: [Row; 7] = [
    (1280, 800, [
        [(1066, 800, None), (1066, 800, None), (1280, 800, Some(4)), (960, 720, Some(3)), (1280, 800, None)],
        [(1280, 800, None), (1280, 800, None), (1280, 800, Some(4)), (1280, 800, Some(4)), (1280, 800, None)],
    ]),
    (1920, 1080, [
        [(1440, 1080, None), (1440, 1080, None), (1600, 1000, Some(5)), (1280, 960, Some(4)), (1920, 1080, None)],
        [(1728, 1080, None), (1728, 1080, None), (1600, 1000, Some(5)), (1600, 1000, Some(5)), (1920, 1080, None)],
    ]),
    (1920, 1200, [
        [(1600, 1200, None), (1600, 1200, None), (1920, 1200, Some(6)), (1600, 1200, Some(5)), (1920, 1200, None)],
        [(1920, 1200, None), (1920, 1200, None), (1920, 1200, Some(6)), (1920, 1200, Some(6)), (1920, 1200, None)],
    ]),
    (2560, 1440, [
        [(1920, 1440, None), (1920, 1440, None), (2240, 1400, Some(7)), (1920, 1440, Some(6)), (2560, 1440, None)],
        [(2304, 1440, None), (2304, 1440, None), (2240, 1400, Some(7)), (2240, 1400, Some(7)), (2560, 1440, None)],
    ]),
    (3440, 1440, [
        [(1920, 1440, None), (1920, 1440, None), (2240, 1400, Some(7)), (1920, 1440, Some(6)), (3440, 1440, None)],
        [(2304, 1440, None), (2304, 1440, None), (2240, 1400, Some(7)), (2240, 1400, Some(7)), (3440, 1440, None)],
    ]),
    (3840, 2160, [
        [(2880, 2160, None), (2880, 2160, None), (3200, 2000, Some(10)), (2880, 2160, Some(9)), (3840, 2160, None)],
        [(3456, 2160, None), (3456, 2160, None), (3200, 2000, Some(10)), (3200, 2000, Some(10)), (3840, 2160, None)],
    ]),
    (5120, 1440, [
        [(1920, 1440, None), (1920, 1440, None), (2240, 1400, Some(7)), (1920, 1440, Some(6)), (5120, 1440, None)],
        [(2304, 1440, None), (2304, 1440, None), (2240, 1400, Some(7)), (2240, 1400, Some(7)), (5120, 1440, None)],
    ]),
];

fn window(physical: UVec2, scale_factor: f32) -> WindowPixels {
    WindowPixels {
        physical,
        scale_factor,
    }
}

fn close(a: Vec2, b: Vec2) -> bool {
    (a - b).abs().max_element() < 1e-3
}

// --- The size tables. -------------------------------------------------------

#[test]
fn the_size_table_holds_for_every_mode_aspect_and_display_scale() {
    for (wx, wy, rows) in TABLE {
        let physical = UVec2::new(wx, wy);
        for (aspect, row) in ASPECTS.into_iter().zip(rows) {
            for (mode, (w, h, factor)) in MODES.into_iter().zip(row) {
                let policy = ScalePolicy::new(mode, aspect);
                let label = format!("{wx}x{wy} {mode:?} {aspect:?}");
                let scaled = policy.scale(physical, FRAME);
                let size = Vec2::new(w as f32, h as f32);
                // Fit is unrounded: the table gives its whole-pixel floor.
                let tolerance = if mode == ScaleMode::Fit { 1.0 } else { 1e-3 };
                let off = (scaled.size - size).abs().max_element();
                assert!(off < tolerance, "{label}: {:?}", scaled.size);
                assert_eq!(scaled.mode, mode, "{label}: no fallback");
                assert_eq!(scaled.factor, factor, "{label}");
                if mode != ScaleMode::Fit {
                    // Whole pixels, placed on a whole pixel, centred.
                    let origin = ((physical.as_vec2() - size) / 2.0).floor();
                    assert_eq!(scaled.origin, origin, "{label}");
                }
                if let Some(k) = factor {
                    assert_eq!(w, k * FRAME.x, "{label}: whole across");
                }
                if mode == ScaleMode::IntegerSquare {
                    assert_eq!(h, factor.unwrap() * FRAME.y, "{label}: whole down");
                }

                // The same physical pixels at every display scale.
                let fit = ScaleFit::new(policy);
                for sf in SCALES {
                    let placed = fit.place(window(physical, sf), FRAME);
                    if policy.is_faithful() {
                        assert_eq!(placed, None, "{label} @{sf}: the 1.0.1 fit draws it");
                        let logical = physical.as_vec2() / sf;
                        let faithful = fitted_display_size(FRAME.x, FRAME.y, logical) * sf;
                        assert!(close(faithful, scaled.size), "{label} @{sf}: {faithful}");
                        continue;
                    }
                    let placed = placed.unwrap_or_else(|| panic!("{label} @{sf}"));
                    assert!(close(placed.size() * sf, scaled.size), "{label} @{sf}");
                    assert!(close(placed.min * sf, scaled.origin), "{label} @{sf}");
                }
            }
        }
    }
}

#[test]
fn integer_square_ignores_the_pixel_aspect() {
    for (wx, wy, _) in TABLE {
        let physical = UVec2::new(wx, wy);
        let dos = ScalePolicy::new(ScaleMode::IntegerSquare, PixelAspect::Dos);
        let square = ScalePolicy::new(ScaleMode::IntegerSquare, PixelAspect::Square);
        assert_eq!(dos.scale(physical, FRAME), square.scale(physical, FRAME));
    }
}

#[test]
fn fit_at_the_dos_aspect_is_the_1_0_1_fit_on_any_window() {
    let policy = ScalePolicy::default();
    assert!(policy.is_faithful());
    for (wx, wy) in [
        (960, 720),
        (1000, 700),
        (1366, 768),
        (801, 1333),
        (3440, 1440),
    ] {
        let physical = UVec2::new(wx, wy);
        let scaled = policy.scale(physical, FRAME);
        let faithful = fitted_display_size(FRAME.x, FRAME.y, physical.as_vec2());
        assert!(close(scaled.size, faithful), "{wx}x{wy}");
        assert!(close(scaled.origin, (physical.as_vec2() - faithful) / 2.0));
    }
}

#[test]
fn sharp_has_the_fit_shape_in_whole_pixels() {
    for (wx, wy) in [(1366, 768), (1000, 700), (1281, 799), (2000, 3000)] {
        let physical = UVec2::new(wx, wy);
        for aspect in ASPECTS {
            let fit = ScalePolicy::new(ScaleMode::Fit, aspect).scale(physical, FRAME);
            let sharp = ScalePolicy::new(ScaleMode::Sharp, aspect).scale(physical, FRAME);
            assert_eq!(sharp.size, sharp.size.floor());
            assert_eq!(sharp.origin, sharp.origin.floor());
            assert!((fit.size - sharp.size).max_element() < 1.0, "{wx}x{wy}");
            assert!(sharp.size.x <= wx as f32 && sharp.size.y <= wy as f32);
        }
    }
}

// --- Fallbacks. -------------------------------------------------------------

#[test]
fn a_window_smaller_than_the_frame_at_scale_one_falls_back_to_fit() {
    for (wx, wy) in [(319, 600), (640, 239), (200, 150)] {
        let physical = UVec2::new(wx, wy);
        for mode in MODES {
            let policy = ScalePolicy::new(mode, PixelAspect::Dos);
            let scaled = policy.scale(physical, FRAME);
            assert_eq!(scaled.mode, ScaleMode::Fit, "{wx}x{wy} {mode:?}");
            // At the DOS aspect that is the 1.0.1 frame itself.
            assert_eq!(
                ScaleFit::new(policy).place(window(physical, 1.0), FRAME),
                None
            );
        }
    }
    // Square pixels need only 320x200, and fall back to the square fit.
    let square = ScalePolicy::new(ScaleMode::IntegerHybrid, PixelAspect::Square);
    assert_eq!(square.scale(UVec2::new(320, 200), FRAME).factor, Some(1));
    let small = UVec2::new(300, 200);
    assert_eq!(square.scale(small, FRAME).mode, ScaleMode::Fit);
    let placed = ScaleFit::new(square)
        .place(window(small, 1.0), FRAME)
        .unwrap();
    assert!(close(placed.size(), Vec2::new(300.0, 187.5)));
}

#[test]
fn the_smallest_window_takes_factor_one() {
    let at_one = UVec2::new(320, 240);
    let hybrid = ScalePolicy::new(ScaleMode::IntegerHybrid, PixelAspect::Dos);
    let scaled = hybrid.scale(at_one, FRAME);
    assert_eq!(
        (scaled.size, scaled.factor),
        (Vec2::new(320.0, 240.0), Some(1))
    );
    let square = ScalePolicy::new(ScaleMode::IntegerSquare, PixelAspect::Dos);
    let scaled = square.scale(at_one, FRAME);
    assert_eq!(scaled.size, Vec2::new(320.0, 200.0));
    assert_eq!(scaled.origin, Vec2::new(0.0, 20.0));
}

// --- The arbiter under a policy. --------------------------------------------

#[test]
fn with_no_policy_the_solver_is_the_foundation_solver() {
    let claims = [PanelClaim {
        kind: PanelKind::Journal,
        side: SidePreference::Auto,
    }];
    let settings = SideSpaceSettings::default();
    for (wx, wy, _) in TABLE {
        let logical = Vec2::new(wx as f32, wy as f32);
        let foundation = solve_side_space(logical, FRAME, &claims, &settings);
        let faithful = ScaleFit::new(ScalePolicy::default());
        for policy in [None, Some(&faithful as &dyn FrameFit)] {
            let solved = solve_frame(
                WindowPixels::logical_at_one(logical),
                FRAME,
                &claims,
                &settings,
                policy,
            );
            assert_eq!(solved, foundation, "{wx}x{wy}");
        }
    }
}

#[test]
fn a_policy_owns_the_frame_with_no_panel() {
    let policy = ScaleFit::new(ScalePolicy::new(ScaleMode::IntegerSquare, PixelAspect::Dos));
    for sf in SCALES {
        let solved = solve_frame(
            window(UVec2::new(1920, 1080), sf),
            FRAME,
            &[],
            &SideSpaceSettings::default(),
            Some(&policy),
        );
        let frame = solved.layout.frame.expect("the policy places the frame");
        assert!(close(frame.size() * sf, Vec2::new(1600.0, 1000.0)));
        assert!(close(frame.min * sf, Vec2::new(160.0, 40.0)));
        assert!(solved.layout.granted.is_empty() && solved.hidden.is_empty());
        let left = solved.layout.left.unwrap();
        let right = solved.layout.right.unwrap();
        assert!(close(left.max, Vec2::new(frame.min.x, 1080.0 / sf)));
        assert!(close(right.min, Vec2::new(frame.max.x, 0.0)));
    }
}

#[test]
fn integer_panels_keep_whole_factors() {
    let policy = ScaleFit::new(ScalePolicy::new(ScaleMode::IntegerSquare, PixelAspect::Dos));
    let claims = [PanelClaim {
        kind: PanelKind::Journal,
        side: SidePreference::Left,
    }];
    for sf in SCALES {
        let solved = solve_frame(
            window(UVec2::new(3440, 1440), sf),
            FRAME,
            &claims,
            &SideSpaceSettings::default(),
            Some(&policy),
        );
        // 320 + 136 = 456 across: 7x fits 3440x1440 (3192x1400).
        let frame = solved.layout.frame.unwrap();
        let (kind, panel) = solved.layout.granted[0];
        assert_eq!(kind, PanelKind::Journal);
        assert!(close(frame.size() * sf, Vec2::new(2240.0, 1400.0)), "@{sf}");
        assert!(close(
            panel.size() * sf,
            Vec2::new(PANEL_WIDTH as f32 * 7.0, 1400.0)
        ));
        assert!(close(panel.max, Vec2::new(frame.min.x, frame.max.y)));
        assert!(close(panel.min * sf, Vec2::new(124.0, 20.0)), "@{sf}");
    }
}

#[test]
fn the_guard_drops_a_panel_that_shrinks_an_integer_frame_too_far() {
    // At 1920x1080 a panel takes integer-square from 5x to 4x: 80% of u0,
    // kept at the default guard and dropped at 90%.
    let policy = ScaleFit::new(ScalePolicy::new(ScaleMode::IntegerSquare, PixelAspect::Dos));
    let claims = [PanelClaim {
        kind: PanelKind::Journal,
        side: SidePreference::Auto,
    }];
    let solve = |guard_percent| {
        solve_frame(
            window(UVec2::new(1920, 1080), 1.0),
            FRAME,
            &claims,
            &SideSpaceSettings {
                guard_percent,
                ..SideSpaceSettings::default()
            },
            Some(&policy),
        )
    };
    let kept = solve(80);
    assert_eq!(kept.layout.granted.len(), 1);
    assert!(close(
        kept.layout.frame.unwrap().size(),
        Vec2::new(1280.0, 800.0)
    ));
    let dropped = solve(90);
    assert_eq!(dropped.hidden, vec![PanelKind::Journal]);
    assert!(close(
        dropped.layout.frame.unwrap().size(),
        Vec2::new(1600.0, 1000.0)
    ));
}

// --- In the app. ------------------------------------------------------------

fn launch(cli: &[(&str, &str)]) -> VisualLaunchConfig {
    let cli: Vec<CliSetting> = cli
        .iter()
        .map(|(key, value)| CliSetting::Set {
            key: (*key).to_string(),
            value: (*value).to_string(),
        })
        .collect();
    VisualLaunchConfig {
        enhancements: resolve_as_shipped(
            ResolveInput {
                stored: None,
                cli: &cli,
                lock: LockState::Off,
                caps: HostCapabilities::ALL,
            },
            &[DISPLAY_P1, DISPLAY_P2, DISPLAY_P8],
        ),
        ..VisualLaunchConfig::classic()
    }
}

fn shell(launch: &VisualLaunchConfig, physical: UVec2, sf: f32) -> Shell {
    let mut shell = Shell::new(
        test_state(open_grid(), 5, 5),
        &Scene::synthetic(),
        launch,
        |_| {},
    );
    shell.app.world_mut().spawn(Window {
        resolution: WindowResolution::new(physical.x as f32, physical.y as f32)
            .with_scale_factor_override(sf),
        ..default()
    });
    shell
}

/// Every sprite's size and position.
fn sprites(shell: &mut Shell) -> Vec<(Vec2, Vec2)> {
    let world = shell.app.world_mut();
    let mut query = world.query::<(&Sprite, &Transform)>();
    query
        .iter(world)
        .map(|(sprite, transform)| {
            (
                sprite.custom_size.unwrap_or_default(),
                transform.translation.truncate(),
            )
        })
        .collect()
}

#[test]
fn fit_adds_nothing_and_keeps_the_1_0_1_frame() {
    for cli in [
        &[][..],
        &[("display.scale_mode", "fit")][..],
        &[("display.pixel_aspect", "dos")][..],
    ] {
        let launch = launch(cli);
        let mut shell = shell(&launch, UVec2::new(1920, 1080), 1.25);
        assert!(!shell.app.world().contains_resource::<FramePolicy>());
        shell.step(&[]);
        shell.step(&[]);
        let fitted = fitted_display_size(FRAME.x, FRAME.y, Vec2::new(1536.0, 864.0));
        for (size, at) in sprites(&mut shell) {
            assert_eq!((size, at), (fitted, Vec2::ZERO), "{cli:?}");
        }
    }
}

#[test]
fn a_launch_policy_sizes_and_places_the_frame_in_physical_pixels() {
    let cases = [
        ("integer-square", "dos", Vec2::new(1600.0, 1000.0)),
        ("integer-hybrid", "dos", Vec2::new(1280.0, 960.0)),
        ("sharp", "square", Vec2::new(1728.0, 1080.0)),
        ("stretch", "dos", Vec2::new(1920.0, 1080.0)),
        ("fit", "square", Vec2::new(1728.0, 1080.0)),
    ];
    for (mode, aspect, physical_size) in cases {
        for sf in SCALES {
            let launch = launch(&[
                ("display.scale_mode", mode),
                ("display.pixel_aspect", aspect),
            ]);
            let mut shell = shell(&launch, UVec2::new(1920, 1080), sf);
            assert!(shell.app.world().contains_resource::<FramePolicy>());
            shell.step(&[]);
            shell.step(&[]);
            let found = sprites(&mut shell);
            assert!(!found.is_empty());
            for (size, at) in found {
                assert!(
                    close(size * sf, physical_size),
                    "{mode} {aspect} @{sf}: {size}"
                );
                // Centred on whole physical pixels: the frame's left edge,
                // from the window's centre, is a whole number of pixels.
                let left = (at.x - size.x / 2.0) * sf + 960.0;
                assert!((left - left.round()).abs() < 1e-2, "{mode} @{sf}: {left}");
            }
        }
    }
}

#[test]
fn a_changed_display_resource_switches_the_policy_and_back_to_1_0_1() {
    let launch = launch(&[("display.scale_mode", "integer-square")]);
    let mut shell = shell(&launch, UVec2::new(1920, 1080), 1.0);
    shell.step(&[]);
    assert!(close(sprites(&mut shell)[0].0, Vec2::new(1600.0, 1000.0)));

    let mut settings = shell.app.world().resource::<Display>().0.clone();
    settings.scale_mode = ScaleMode::Stretch;
    shell
        .app
        .world_mut()
        .insert_resource(Display(settings.clone()));
    shell.step(&[]);
    assert!(close(sprites(&mut shell)[0].0, Vec2::new(1920.0, 1080.0)));

    settings.scale_mode = ScaleMode::Fit;
    shell.app.world_mut().insert_resource(Display(settings));
    shell.step(&[]);
    shell.step(&[]);
    let fitted = fitted_display_size(FRAME.x, FRAME.y, Vec2::new(1920.0, 1080.0));
    assert_eq!(sprites(&mut shell)[0], (fitted, Vec2::ZERO));
}

#[test]
fn a_resize_refits_the_frame() {
    let launch = launch(&[("display.scale_mode", "integer-square")]);
    let mut shell = shell(&launch, UVec2::new(1920, 1080), 1.0);
    shell.step(&[]);
    assert!(close(sprites(&mut shell)[0].0, Vec2::new(1600.0, 1000.0)));
    let world = shell.app.world_mut();
    let mut windows = world.query::<&mut Window>();
    windows
        .single_mut(world)
        .unwrap()
        .resolution
        .set_physical_resolution(2560, 1440);
    shell.step(&[]);
    assert!(close(sprites(&mut shell)[0].0, Vec2::new(2240.0, 1400.0)));
}

// --- Sharp-bilinear: the `[prescale]` step (M1-03). ---------------------

/// The canvas rule's width for a window (2.2.11): 240 rows at 1.2, or 216
/// at 1.25 on a height that is a multiple of 1080.
fn named_canvas(window: UVec2) -> UVec2 {
    let (rows, aspect) = if window.y.is_multiple_of(1080) {
        (216.0, 1.25)
    } else {
        (240.0, 1.2)
    };
    let width = 16.0 * (rows * aspect * window.x as f32 / window.y as f32 / 16.0).floor();
    UVec2::new(width as u32, rows as u32)
}

#[test]
fn the_prescale_factors_are_exact_on_the_named_canvases() {
    for (window, canvas, factors) in [
        (
            UVec2::new(2560, 1440),
            UVec2::new(512, 240),
            UVec2::new(5, 6),
        ),
        (
            UVec2::new(1920, 1080),
            UVec2::new(480, 216),
            UVec2::new(4, 5),
        ),
        (
            UVec2::new(3440, 1440),
            UVec2::new(688, 240),
            UVec2::new(5, 6),
        ),
        (
            UVec2::new(2560, 1080),
            UVec2::new(640, 216),
            UVec2::new(4, 5),
        ),
    ] {
        assert_eq!(named_canvas(window), canvas, "{window}");
        let found = prescale_factors(window.as_vec2(), canvas);
        assert_eq!(found, factors, "{window}");
        assert_eq!(canvas * found, window, "{window}: exact, so sharp is Fit");
    }
    // 2160 rows would be 8x10; the cap holds both axes at 8.
    let window = UVec2::new(3840, 2160);
    assert_eq!(named_canvas(window), UVec2::new(480, 216));
    assert_eq!(
        prescale_factors(window.as_vec2(), UVec2::new(480, 216)),
        UVec2::new(8, 8)
    );
}

#[test]
fn the_prescale_factors_of_the_frame_follow_the_sharp_size() {
    for (wx, wy, sizes) in TABLE {
        for (aspect, row) in ASPECTS.into_iter().zip(sizes) {
            let (w, h, _) = row[1];
            let factors = prescale_factors(Vec2::new(w as f32, h as f32), FRAME);
            let expect = UVec2::new(
                (w / FRAME.x).clamp(1, PRESCALE_CAP),
                (h / FRAME.y).clamp(1, PRESCALE_CAP),
            );
            assert_eq!(factors, expect, "{wx}x{wy} {aspect:?}");
        }
    }
    // A hair under a whole multiple is that multiple; under the image, 1.
    assert_eq!(
        prescale_factors(Vec2::new(1279.9999, 999.9999), FRAME),
        UVec2::new(4, 5)
    );
    assert_eq!(prescale_factors(Vec2::new(200.0, 100.0), FRAME), UVec2::ONE);
    assert_eq!(prescale_factors(Vec2::ZERO, FRAME), UVec2::ONE);
    assert_eq!(
        prescale_factors(Vec2::new(f32::NAN, 9000.0), FRAME),
        UVec2::new(1, 8)
    );
}

/// A small image whose every pixel differs.
fn pattern(size: UVec2) -> Vec<u8> {
    (0..size.x * size.y)
        .flat_map(|i| [(i * 37) as u8, (i * 11 + 3) as u8, (i * 5 + 7) as u8, 0xff])
        .collect()
}

/// Nearest-neighbour resampling the long way: each output pixel samples the
/// source pixel it falls in.
fn nearest_resample(rgba: &[u8], size: UVec2, out: UVec2) -> Vec<u8> {
    let mut pixels = Vec::new();
    for y in 0..out.y {
        for x in 0..out.x {
            let sx = (x as f32 + 0.5) * size.x as f32 / out.x as f32;
            let sy = (y as f32 + 0.5) * size.y as f32 / out.y as f32;
            let offset = ((sy as u32 * size.x + sx as u32) * 4) as usize;
            pixels.extend_from_slice(&rgba[offset..offset + 4]);
        }
    }
    pixels
}

#[test]
fn prescale_pixels_equal_nearest_expansion() {
    let size = UVec2::new(7, 5);
    let source = pattern(size);
    let mut out = Vec::new();
    for factors in [
        UVec2::ONE,
        UVec2::new(3, 2),
        UVec2::new(4, 5),
        UVec2::new(5, 6),
        UVec2::new(8, 8),
    ] {
        prescale_nearest(&source, size, factors, &mut out);
        let expanded = size * factors;
        assert_eq!(out.len(), (expanded.x * expanded.y * 4) as usize);
        for y in 0..expanded.y {
            for x in 0..expanded.x {
                let at = ((y * expanded.x + x) * 4) as usize;
                let from = (((y / factors.y) * size.x + x / factors.x) * 4) as usize;
                assert_eq!(out[at..at + 4], source[from..from + 4], "{factors} {x},{y}");
            }
        }
        assert_eq!(out, nearest_resample(&source, size, expanded), "{factors}");
    }
}

fn sharp_shell(physical: UVec2, sf: f32) -> Shell {
    shell(&launch(&[("display.scale_mode", "sharp")]), physical, sf)
}

/// The frame sprite's image and the faithful one: sizes, samplers, bytes.
fn presented(shell: &mut Shell) -> (UVec2, String, Vec<u8>, UVec2, String, Vec<u8>) {
    let faithful = shell.visual().image_handle.clone();
    let world = shell.app.world_mut();
    let shown = world
        .query::<&Sprite>()
        .single(world)
        .unwrap()
        .image
        .clone();
    let images = world.resource::<Assets<Image>>();
    let shown = images.get(&shown).unwrap();
    let faithful = images.get(&faithful).unwrap();
    (
        shown.size(),
        format!("{:?}", shown.sampler),
        shown.data.clone().unwrap(),
        faithful.size(),
        format!("{:?}", faithful.sampler),
        faithful.data.clone().unwrap(),
    )
}

#[test]
fn sharp_registers_the_prescale_and_no_other_mode_does() {
    for mode in MODES {
        let launch = launch(&[("display.scale_mode", mode.key())]);
        let shell = shell(&launch, UVec2::new(1920, 1080), 1.0);
        let steps = shell
            .app
            .world()
            .get_resource::<PresentSteps>()
            .map(PresentSteps::orders);
        let expect = (mode == ScaleMode::Sharp).then(|| vec![StepOrder::PRESCALE]);
        assert_eq!(steps, expect, "{mode:?}");
    }
}

#[test]
fn sharp_shows_a_nearest_expansion_with_a_linear_sampler() {
    let nearest = format!("{:?}", bevy::image::ImageSampler::nearest());
    let linear = format!("{:?}", bevy::image::ImageSampler::linear());
    // 1920x1080 at every display scale: the frame is 1440x1080 physical,
    // so 4x5. 1280x800 sharp is 1066x800 physical: 3x4.
    for (window, sf, factors) in [
        (UVec2::new(1920, 1080), 1.0, UVec2::new(4, 5)),
        (UVec2::new(1920, 1080), 1.25, UVec2::new(4, 5)),
        (UVec2::new(1920, 1080), 1.5, UVec2::new(4, 5)),
        (UVec2::new(1280, 800), 1.0, UVec2::new(3, 4)),
        (UVec2::new(3840, 2160), 1.0, UVec2::new(8, 8)),
    ] {
        let mut shell = sharp_shell(window, sf);
        shell.step(&[]);
        shell.step(&[]);
        let (size, sampler, bytes, faithful_size, faithful_sampler, faithful) =
            presented(&mut shell);
        assert_eq!(faithful_size, FRAME);
        assert_eq!(
            faithful_sampler, nearest,
            "the faithful image keeps nearest"
        );
        assert_eq!(size, FRAME * factors, "{window} @{sf}");
        assert_eq!(sampler, linear, "{window} @{sf}");
        let mut expect = Vec::new();
        prescale_nearest(&faithful, FRAME, factors, &mut expect);
        assert!(bytes == expect, "{window} @{sf}: not the nearest expansion");
    }
}

#[test]
fn the_prescale_follows_the_game_a_resize_and_a_mode_change() {
    let mut shell = sharp_shell(UVec2::new(1920, 1080), 1.0);
    shell.step(&[]);
    let (_, _, before, ..) = presented(&mut shell);
    // The player walks: the presented copy follows the faithful frame.
    let mut moved = false;
    for frame in 0..30 {
        let events = if frame == 2 {
            vec![crate::enh::equivalence::TimedKey {
                key: KeyCode::ArrowUp,
                state: bevy::input::ButtonState::Pressed,
                repeat: false,
                owner: crate::enh::equivalence::Owner::Game,
            }]
        } else if frame == 3 {
            vec![crate::enh::equivalence::TimedKey {
                key: KeyCode::ArrowUp,
                state: bevy::input::ButtonState::Released,
                repeat: false,
                owner: crate::enh::equivalence::Owner::Game,
            }]
        } else {
            Vec::new()
        };
        shell.step(&events);
        let (size, _, bytes, _, _, faithful) = presented(&mut shell);
        let mut expect = Vec::new();
        prescale_nearest(&faithful, FRAME, size / FRAME, &mut expect);
        assert!(bytes == expect, "frame {frame}");
        moved |= bytes != before;
    }
    assert!(moved, "the walk must change the frame");

    // A resize: new factors.
    let world = shell.app.world_mut();
    let mut windows = world.query::<&mut Window>();
    windows
        .single_mut(world)
        .unwrap()
        .resolution
        .set_physical_resolution(2560, 1440);
    shell.step(&[]);
    shell.step(&[]);
    // 2560x1440 sharp: 1920x1440, 6x7.
    assert_eq!(presented(&mut shell).0, FRAME * UVec2::new(6, 7));

    // Integer-square: the step goes and the sprite shows the faithful image.
    let mut settings = shell.app.world().resource::<Display>().0.clone();
    settings.scale_mode = ScaleMode::IntegerSquare;
    shell
        .app
        .world_mut()
        .insert_resource(Display(settings.clone()));
    shell.step(&[]);
    shell.step(&[]);
    assert!(shell.app.world().resource::<PresentSteps>().is_empty());
    let faithful = shell.visual().image_handle.clone();
    let world = shell.app.world_mut();
    assert_eq!(
        world.query::<&Sprite>().single(world).unwrap().image,
        faithful
    );

    // And back to sharp.
    settings.scale_mode = ScaleMode::Sharp;
    shell.app.world_mut().insert_resource(Display(settings));
    shell.step(&[]);
    shell.step(&[]);
    assert_eq!(presented(&mut shell).0, FRAME * UVec2::new(6, 7));
}

#[test]
fn sharp_leaves_the_game_alone() {
    use crate::enh::equivalence::{Timeline, check_shell_equivalence};
    let launch = launch(&[("display.scale_mode", "sharp")]);
    let mut timeline = Timeline::new(60);
    timeline
        .game_tap(5, KeyCode::ArrowUp)
        .game_tap(20, KeyCode::ArrowLeft)
        .game_tap(35, KeyCode::ArrowDown);
    let mismatches = check_shell_equivalence(
        &test_state(open_grid(), 5, 5),
        &Scene::synthetic(),
        &timeline,
        &launch,
        |app: &mut App| {
            app.world_mut().spawn(Window {
                resolution: WindowResolution::new(1920.0, 1080.0),
                ..default()
            });
        },
    );
    assert_eq!(mismatches, []);
}

//! The `[brightness]` step (design 9.3.3; build plan M2-03): the region per
//! scope and per map kind, changes only inside it, the faithful image left
//! alone, the grow-left text kept at full colour, the place before the
//! prescale, no step at 100, the menu's changes followed, and the cost.

use std::time::{Duration, Instant};

use bevy::window::WindowResolution;
use u5_runtime::enhancements::{
    BrightnessScope, CliSetting, HostCapabilities, LockState, PixelAspect, ResolveInput, resolve,
};
use u5_runtime::test_fixtures::{open_grid, synthetic_tile_atlas, test_state};
use u5_runtime::{PeekLightPolicy, PlayState, TileGraphicsDepth, hash_bytes};

use super::*;
use crate::display::crawl::layer::{CapSprite, CrawlCell, CrawlLine, CrawlRows, LineKind};
use crate::display::effects::TextMask;
use crate::display::grow::column::{COLUMN_RULE_X, ColumnView};
use crate::display::grow::crawl_paint::CrawlPaint;
use crate::display::grow::{Canvas, ComposeOptions, compose_for_state_mapped};
use crate::display::scale::prescale_nearest;
use crate::enh::VisualLaunchConfig;
use crate::enh::equivalence::{Scene, Shell, Timeline, check_shell_equivalence};
use crate::{EGA_PALETTE_RGB, VisualState};

const KINDS: [MapViewKind; 4] = [
    MapViewKind::World,
    MapViewKind::Town,
    MapViewKind::Combat,
    MapViewKind::Dungeon,
];

fn light(percent: u8, scope: BrightnessScope) -> DisplayLight {
    DisplayLight {
        brightness: percent,
        scope,
        ..DisplayLight::default()
    }
}

fn step(percent: u8, scope: BrightnessScope) -> BrightnessStep {
    BrightnessStep::of(&light(percent, scope)).expect("a step below 100")
}

/// A frame of EGA colours, distinct from pixel to pixel, black included,
/// with a varied alpha.
fn pattern(size: UVec2, seed: usize) -> Vec<u8> {
    (0..(size.x * size.y) as usize)
        .flat_map(|i| {
            let [r, g, b] = EGA_PALETTE_RGB[(i * 7 + i / size.x as usize + seed) % 16];
            [r, g, b, 0xff - (i % 3) as u8]
        })
        .collect()
}

fn frame_of(size: UVec2, rgba: Vec<u8>) -> PresentedFrame {
    PresentedFrame {
        size,
        rgba,
        ..PresentedFrame::default()
    }
}

/// Every pixel inside `region` is `lut` of the original (alpha kept) and
/// every pixel outside it is the original, byte for byte.
fn assert_dimmed_exactly(
    before: &[u8],
    after: &[u8],
    size: UVec2,
    region: Option<&MapRegion>,
    lut: &BrightnessLut,
    context: &str,
) {
    assert_eq!(before.len(), after.len(), "{context}");
    for y in 0..size.y {
        for x in 0..size.x {
            let at = ((y * size.x + x) * 4) as usize;
            let (old, new) = (&before[at..at + 4], &after[at..at + 4]);
            let expected = if region.is_some_and(|region| region.contains(x, y)) {
                [
                    lut.apply(old[0]),
                    lut.apply(old[1]),
                    lut.apply(old[2]),
                    old[3],
                ]
            } else {
                [old[0], old[1], old[2], old[3]]
            };
            assert_eq!(new, expected, "{context}: pixel ({x}, {y})");
        }
    }
}

// --- The region (9.3.3 step 1). --------------------------------------------

#[test]
fn scope_map_is_the_viewport_on_every_map_kind_and_nothing_elsewhere() {
    for kind in KINDS {
        let region =
            brightness_region(BrightnessScope::Map, Some(kind), FRAME, None).expect("a map shows");
        assert_eq!(region.rects, [(8, 8, 183, 183)], "{kind:?}");
        assert_eq!(region.count(), 176 * 176, "{kind:?}");
        for (x, y, inside) in [
            (8, 8, true),
            (183, 183, true),
            (7, 8, false),
            (8, 7, false),
            (184, 100, false),
            (100, 184, false),
            (250, 100, false),
        ] {
            assert_eq!(region.contains(x, y), inside, "{kind:?} ({x}, {y})");
        }
    }
    // No map (the text panel, the shrine, the cinematics, the intro): the
    // copy passes through.
    assert_eq!(
        brightness_region(BrightnessScope::Map, None, FRAME, None),
        None
    );
}

#[test]
fn scope_frame_is_the_whole_copy_with_or_without_a_map() {
    for kind in KINDS.map(Some).into_iter().chain([None]) {
        for size in [FRAME, UVec2::new(688, 240)] {
            let region = brightness_region(BrightnessScope::Frame, kind, size, None).unwrap();
            assert_eq!(region.count(), (size.x * size.y) as usize, "{kind:?}");
        }
    }
}

#[test]
fn scope_map_takes_the_region_an_earlier_step_set() {
    let size = UVec2::new(688, 240);
    let mut set = MapRegion::rect(size, (152, 16, 679, 231));
    set.exclude(200, 100);
    let region = brightness_region(
        BrightnessScope::Map,
        Some(MapViewKind::World),
        size,
        Some(&set),
    );
    assert_eq!(region.as_ref(), Some(&set));
    // A region made for another size, or a resized copy with none: leave
    // the copy alone.
    let other = MapRegion::rect(FRAME, VIEWPORT);
    assert_eq!(
        brightness_region(
            BrightnessScope::Map,
            Some(MapViewKind::World),
            size,
            Some(&other)
        ),
        None
    );
    assert_eq!(
        brightness_region(BrightnessScope::Map, Some(MapViewKind::World), size, None),
        None
    );
    // Without a map, a set region does not bring it back.
    assert_eq!(
        brightness_region(BrightnessScope::Map, None, size, Some(&set)),
        None
    );
}

#[test]
fn a_regions_runs_skip_the_excluded_pixels() {
    let size = UVec2::new(16, 4);
    let mut region = MapRegion::rect(size, (2, 1, 9, 2));
    region.exclude(4, 1);
    region.exclude(5, 1);
    region.exclude(9, 2);
    region.exclude(40, 40);
    let mut runs = Vec::new();
    region.for_each_run(|y, x0, x1| runs.push((y, x0, x1)));
    assert_eq!(runs, [(1, 2, 4), (1, 6, 10), (2, 2, 9)]);
    assert_eq!(region.count(), 2 + 4 + 7);
    assert!(region.contains(3, 1) && !region.contains(4, 1) && !region.contains(1, 1));
}

// --- The scale. --------------------------------------------------------------

#[test]
fn the_step_changes_only_the_region_per_scope_and_kind() {
    let lut70 = BrightnessLut::for_percent(70);
    let original = pattern(FRAME, 3);
    for scope in [BrightnessScope::Map, BrightnessScope::Frame] {
        for kind in KINDS.map(Some).into_iter().chain([None]) {
            let mut frame = frame_of(FRAME, original.clone());
            step(70, scope).dim(kind, &mut frame);
            let region = brightness_region(scope, kind, FRAME, None);
            assert_dimmed_exactly(
                &original,
                &frame.rgba,
                FRAME,
                region.as_ref(),
                &lut70,
                &format!("{scope:?} {kind:?}"),
            );
            let changed = original != frame.rgba;
            assert_eq!(changed, region.is_some(), "{scope:?} {kind:?}");
        }
    }
}

#[test]
fn the_grey_levels_follow_the_table_and_black_stays_black() {
    // 9.2.2: 0x55, 0xAA and white at 70%.
    let step = step(70, BrightnessScope::Frame);
    for (from, to) in [(0x00, 0), (0x55, 71), (0xAA, 145), (0xFF, 218)] {
        assert_eq!(step.lut().apply(from), to);
    }
}

#[test]
fn at_100_there_is_no_step() {
    assert!(BrightnessStep::of(&light(100, BrightnessScope::Map)).is_none());
    assert!(BrightnessStep::of(&light(100, BrightnessScope::Frame)).is_none());
    assert!(BrightnessStep::of(&DisplayLight::default()).is_none());
    for percent in [25, 55, 70, 85, 99] {
        assert!(BrightnessStep::of(&light(percent, BrightnessScope::Map)).is_some());
    }
}

fn input_for<'a>(faithful: &'a [u8], state: Option<&'a PlayState>) -> PresentInput<'a> {
    PresentInput {
        faithful,
        size: FRAME,
        faithful_hash: 0,
        state,
        atlas: None,
        font: None,
        window: UVec2::ZERO,
        scale_factor: 1.0,
        side: None,
        sweep_settling: false,
    }
}

#[test]
fn the_revision_follows_the_percent_the_scope_and_the_map_kind() {
    let world = test_state(open_grid(), 5, 5);
    assert!(world.map_view_kind().is_some_and(MapViewKind::is_top_down));
    let mut combat = world.clone();
    combat.combat_active = true;
    assert_eq!(combat.map_view_kind(), Some(MapViewKind::Combat));
    let mut fixed = world.clone();
    fixed.cutscene_black_viewport = true;
    assert_eq!(fixed.map_view_kind(), None);
    let faithful = pattern(FRAME, 0);
    let input = |state| input_for(&faithful, state);
    let mut seen = std::collections::HashSet::new();
    for (percent, scope) in [
        (70, BrightnessScope::Map),
        (55, BrightnessScope::Map),
        (70, BrightnessScope::Frame),
    ] {
        let mut step = step(percent, scope);
        let fixed_revision = step.revision(&input(Some(&fixed)));
        // The intro shows no map either: the same revision as a fixed
        // screen.
        assert_eq!(step.revision(&input(None)), fixed_revision);
        for state in [&world, &combat, &fixed] {
            let revision = step.revision(&input(Some(state)));
            assert_eq!(revision, step.revision(&input(Some(state))), "stable");
            seen.insert(revision);
        }
    }
    assert_eq!(seen.len(), 3 * 3);
}

/// M2 gate review: the White potion's visibility sweep draws the map it
/// sweeps over, so scope `map` keeps dimming the viewport through it rather
/// than letting the map flash to full colour for the sweep's frames.
#[test]
fn a_visibility_sweep_keeps_the_map_dimmed() {
    let mut swept = test_state(open_grid(), 5, 5);
    swept.visibility_sweep = Some(u5_runtime::VisibilitySweep {
        frames_remaining: 7,
        pause_bios_ticks_per_frame: 1,
        center_x: 5,
        center_y: 5,
        visible_cells: [true; u5_runtime::VIEWPORT_SIDE * u5_runtime::VIEWPORT_SIDE],
    });
    assert_eq!(swept.map_view_kind(), None);
    let plain = test_state(open_grid(), 5, 5);
    let faithful = pattern(FRAME, 3);
    let mut step = step(70, BrightnessScope::Map);
    let dimmed = |step: &mut BrightnessStep, state: &PlayState| {
        let mut frame = frame_of(FRAME, faithful.clone());
        step.apply(&input_for(&faithful, Some(state)), &mut frame);
        frame.rgba
    };
    let during = dimmed(&mut step, &swept);
    assert_ne!(during, faithful, "the sweep's map is dimmed");
    assert_eq!(during, dimmed(&mut step, &plain), "as the map is");
    assert_eq!(
        step.revision(&input_for(&faithful, Some(&swept))),
        step.revision(&input_for(&faithful, Some(&plain))),
        "no repaint as the sweep starts or ends"
    );
}

// --- In the shell. -----------------------------------------------------------

fn launch(cli: &[CliSetting]) -> VisualLaunchConfig {
    VisualLaunchConfig {
        enhancements: resolve(ResolveInput {
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
        key: key.into(),
        value: value.into(),
    }
}

fn spawn_window(app: &mut App, width: f32, height: f32) {
    app.world_mut().spawn(Window {
        resolution: WindowResolution::new(width, height),
        ..default()
    });
}

fn shell(launch: &VisualLaunchConfig, width: f32, height: f32) -> Shell {
    let mut shell = Shell::new(
        test_state(open_grid(), 5, 5),
        &Scene::synthetic(),
        launch,
        |_| {},
    );
    spawn_window(&mut shell.app, width, height);
    shell
}

fn orders(shell: &Shell) -> Option<Vec<StepOrder>> {
    shell
        .app
        .world()
        .get_resource::<PresentSteps>()
        .map(PresentSteps::orders)
}

/// The faithful frame's bytes, and the frame sprite's image's bytes and
/// size.
fn images(shell: &mut Shell) -> (Vec<u8>, Vec<u8>, UVec2) {
    let world = shell.app.world_mut();
    let faithful = world.resource::<VisualState>().image_handle.clone();
    let mut sprites = world.query::<&Sprite>();
    let shown = sprites.single(world).unwrap().image.clone();
    let assets = world.resource::<Assets<Image>>();
    let bytes = |handle: &Handle<Image>| assets.get(handle).unwrap().data.clone().unwrap();
    let size = assets.get(&shown).unwrap().size();
    (bytes(&faithful), bytes(&shown), size)
}

/// A short walk on the open grid.
fn walk(frames: usize) -> Timeline {
    let mut timeline = Timeline::new(frames);
    timeline
        .game_tap(5, KeyCode::ArrowRight)
        .game_tap(15, KeyCode::ArrowDown)
        .game_tap(25, KeyCode::ArrowLeft);
    timeline
}

#[test]
fn the_step_is_registered_only_below_100() {
    for cli in [
        vec![],
        vec![set("display.brightness", "100")],
        vec![CliSetting::Disable("display.brightness".into())],
        vec![set("display.brightness_scope", "frame")],
    ] {
        let launch = launch(&cli);
        assert!(launch.enhancements.notes.is_empty(), "{cli:?}");
        let shell = shell(&launch, 1280.0, 960.0);
        assert_eq!(orders(&shell), None, "{cli:?}");
    }
    for cli in [
        vec![set("display.brightness", "70")],
        vec![CliSetting::Enable("display.brightness".into())],
        vec![
            set("display.brightness", "25"),
            set("display.brightness_scope", "frame"),
        ],
    ] {
        let launch = launch(&cli);
        assert!(launch.enhancements.notes.is_empty(), "{cli:?}");
        let shell = shell(&launch, 1280.0, 960.0);
        assert_eq!(orders(&shell), Some(vec![StepOrder::BRIGHTNESS]), "{cli:?}");
    }
    // With the canvas and the sharp prescale, it sits between them.
    let all = launch(&[
        set("display.brightness", "70"),
        CliSetting::Enable("display.grow_left".into()),
        set("display.scale_mode", "sharp"),
    ]);
    assert_eq!(
        orders(&shell(&all, 1920.0, 1080.0)),
        Some(vec![
            StepOrder::GROW_LEFT_COMPOSE,
            StepOrder::BRIGHTNESS,
            StepOrder::PRESCALE
        ])
    );
}

#[test]
fn the_faithful_image_is_unchanged_and_the_copy_dims_the_viewport() {
    let mut off = shell(&VisualLaunchConfig::classic(), 1280.0, 960.0);
    let mut on = shell(&launch(&[set("display.brightness", "70")]), 1280.0, 960.0);
    let lut = BrightnessLut::for_percent(70);
    let region = MapRegion::rect(FRAME, VIEWPORT);
    let timeline = walk(40);
    for index in 0..timeline.len() {
        off.step(timeline.frame(index));
        on.step(timeline.frame(index));
        let (faithful_off, _, _) = images(&mut off);
        let (faithful_on, shown, size) = images(&mut on);
        assert_eq!(faithful_on, faithful_off, "frame {index}: faithful bytes");
        assert_eq!(hash_bytes(&faithful_on), hash_bytes(&faithful_off));
        assert_eq!(size, FRAME);
        assert_dimmed_exactly(
            &faithful_on,
            &shown,
            FRAME,
            Some(&region),
            &lut,
            &format!("frame {index}"),
        );
    }
}

#[test]
fn the_game_runs_the_same_with_the_step_on() {
    let start = test_state(open_grid(), 5, 5);
    for cli in [
        vec![set("display.brightness", "70")],
        vec![
            set("display.brightness", "40"),
            set("display.brightness_scope", "frame"),
        ],
    ] {
        let mismatches = check_shell_equivalence(
            &start,
            &Scene::synthetic(),
            &walk(40),
            &launch(&cli),
            |app| spawn_window(app, 1280.0, 960.0),
        );
        assert_eq!(mismatches, [], "{cli:?}");
    }
}

/// The copy dims after the canvas and before the prescale: with sharp
/// scaling, the presented image is the prescale of the dimmed frame.
#[test]
fn the_copy_dims_before_the_prescale() {
    let launch = launch(&[
        set("display.brightness", "55"),
        set("display.scale_mode", "sharp"),
    ]);
    let mut shell = shell(&launch, 1280.0, 960.0);
    let lut = BrightnessLut::for_percent(55);
    for index in 0..3 {
        shell.step(&[]);
        let (faithful, shown, size) = images(&mut shell);
        let factors = size / FRAME;
        assert!(factors.x > 1 && factors.y > 1, "frame {index}: {size}");
        let mut dimmed = frame_of(FRAME, faithful.clone());
        dim_region(&lut, &MapRegion::rect(FRAME, VIEWPORT), &mut dimmed);
        assert_ne!(dimmed.rgba, faithful);
        let mut expected = Vec::new();
        prescale_nearest(&dimmed.rgba, FRAME, factors, &mut expected);
        assert_eq!(shown, expected, "frame {index}");
    }
}

#[test]
fn the_menus_changes_are_followed() {
    let mut shell = shell(&launch(&[set("display.brightness", "70")]), 1280.0, 960.0);
    shell.step(&[]);
    let (faithful, shown, _) = images(&mut shell);
    assert_ne!(faithful, shown);
    let change = |shell: &mut Shell, percent: u8, scope: BrightnessScope| {
        {
            let mut display = shell.app.world_mut().resource_mut::<Display>();
            display.0.light.brightness = percent;
            display.0.light.scope = scope;
        }
        shell.step(&[]);
        shell.step(&[]);
    };
    change(&mut shell, 40, BrightnessScope::Frame);
    assert_eq!(orders(&shell), Some(vec![StepOrder::BRIGHTNESS]));
    let (faithful, shown, _) = images(&mut shell);
    assert_dimmed_exactly(
        &faithful,
        &shown,
        FRAME,
        Some(&MapRegion::rect(FRAME, (0, 0, 319, 199))),
        &BrightnessLut::for_percent(40),
        "40% frame",
    );
    change(&mut shell, 100, BrightnessScope::Frame);
    assert_eq!(orders(&shell), Some(vec![]));
    let (faithful, shown, _) = images(&mut shell);
    assert_eq!(faithful, shown, "100: the faithful image again");
    change(&mut shell, 85, BrightnessScope::Map);
    assert_eq!(orders(&shell), Some(vec![StepOrder::BRIGHTNESS]));
    let (faithful, shown, _) = images(&mut shell);
    assert_dimmed_exactly(
        &faithful,
        &shown,
        FRAME,
        Some(&MapRegion::rect(FRAME, VIEWPORT)),
        &BrightnessLut::for_percent(85),
        "85% map",
    );
}

// --- Under the grow-left canvas (D-M2.5). ------------------------------------

fn crawl_rows() -> CrawlRows {
    let line = |kind: LineKind, cells: usize| CrawlLine {
        kind,
        age: 0,
        cells: (0..cells)
            .map(|cell| CrawlCell {
                cell: cell as u8,
                byte: b'x',
                mask: [0x3c, 0x42, 0x81, 0xff, 0x81, 0x42, 0x3c, 0x18],
                typed: cell % 2 == 0,
            })
            .collect(),
        frame_row: 11,
    };
    let mut cap: CapSprite = [[None; 8]; 8];
    for (row, pixels) in cap.iter_mut().enumerate() {
        pixels[0] = Some(1);
        pixels[row] = Some(15);
    }
    CrawlRows {
        lines: vec![line(LineKind::Text, 20), line(LineKind::Live, 12)],
        cursor: None,
        cap: Some(cap),
        ..CrawlRows::default()
    }
}

fn compose(
    state: &PlayState,
    canvas: &Canvas,
    rows: &CrawlRows,
    column: ColumnView,
) -> (Vec<u8>, MapRegion) {
    let atlas = synthetic_tile_atlas(TileGraphicsDepth::Ega16);
    let frame = pattern(FRAME, 5);
    compose_for_state_mapped(
        &frame,
        state,
        &atlas,
        canvas,
        ComposeOptions {
            light: PeekLightPolicy::Scaled,
            bands: None,
            crawl: Some(CrawlPaint {
                rows,
                crawl: true,
                typed_colour: true,
            }),
            column,
            centre_party: true,
            composite: None,
        },
    )
    .expect("a canvas on the open grid")
}

#[test]
fn under_grow_left_the_text_and_column_keep_their_pixels() {
    let state = test_state(open_grid(), 5, 5);
    let canvas = Canvas::for_window(3440, 1440, PixelAspect::Dos).unwrap();
    assert_eq!((canvas.w, canvas.h), (688, 240));
    let size = UVec2::new(canvas.w, canvas.h);
    let rows = crawl_rows();
    let lut = BrightnessLut::for_percent(70);
    let (x0, y0, x1, y1) = canvas.map_window();
    let text = TextMask::of_crawl(
        &canvas,
        CrawlPaint {
            rows: &rows,
            crawl: true,
            typed_colour: true,
        },
    );
    for (name, column) in [
        ("closed", ColumnView::default()),
        (
            "open",
            ColumnView {
                open: true,
                ..ColumnView::default()
            },
        ),
        (
            "opening, step 3",
            ColumnView {
                open: true,
                step: Some(3),
                page_mode: false,
            },
        ),
        (
            "closing, step 5",
            ColumnView {
                open: false,
                step: Some(5),
                page_mode: false,
            },
        ),
    ] {
        let (rgba, region) = compose(&state, &canvas, &rows, column);
        assert_eq!(region.size, size, "{name}");
        let mut frame = PresentedFrame {
            size,
            rgba: rgba.clone(),
            smooth: false,
            map_region: Some(region.clone()),
        };
        step(70, BrightnessScope::Map).dim(Some(MapViewKind::World), &mut frame);
        assert_dimmed_exactly(&rgba, &frame.rgba, size, Some(&region), &lut, name);
        let mut text_pixels = 0;
        for y in 0..canvas.h {
            for x in 0..canvas.w {
                let in_window = (x0..=x1).contains(&x) && (y0..=y1).contains(&y);
                // The bands (ribbon and tabs) and the rules are not map.
                if !in_window {
                    assert!(!region.contains(x, y), "{name}: ({x}, {y}) off the map");
                    continue;
                }
                // Nor are the crawl's glyphs and outlines, nor the shown
                // column.
                let collapsed = !column.open && column.step.is_none();
                if collapsed && text.contains(x, y) {
                    text_pixels += 1;
                    assert!(!region.contains(x, y), "{name}: text at ({x}, {y})");
                }
                if column.open && column.step.is_none() && x <= COLUMN_RULE_X {
                    assert!(!region.contains(x, y), "{name}: column at ({x}, {y})");
                }
            }
        }
        if name == "closed" {
            assert!(text_pixels > 100, "{name}: the crawl drew text");
        }
        // The map is dimmed: at least the core's coloured pixels changed
        // (the synthetic ring is mostly black).
        let changed = (y0..=y1)
            .flat_map(|y| (x0..=x1).map(move |x| (x, y)))
            .filter(|&(x, y)| {
                let at = ((y * canvas.w + x) * 4) as usize;
                frame.rgba[at..at + 3] != rgba[at..at + 3]
            })
            .count();
        assert!(changed > 176 * 176 * 9 / 10, "{name}: {changed} changed");
    }
    // The end cap at x 8..15 on the live row is not map either.
    let (_, region) = compose(&state, &canvas, &rows, ColumnView::default());
    // Its first column is drawn on all eight rows; the text starts at x 16,
    // so its outline never reaches x 8.
    let cap_rows = (y0..=y1).filter(|&y| !region.contains(8, y)).count();
    assert_eq!(cap_rows, 8);
}

// --- The cost. ---------------------------------------------------------------

/// 9.3.3: well under a millisecond on the 21:9 canvas (688x240) in the
/// player's (optimised) build: `cargo test --release -p u5-bevy --lib
/// light_tests`. An unoptimised build only reports its numbers.
#[test]
fn the_step_costs_under_a_millisecond_at_688x240() {
    let size = UVec2::new(688, 240);
    let canvas = Canvas::for_window(3440, 1440, PixelAspect::Dos).unwrap();
    assert_eq!(UVec2::new(canvas.w, canvas.h), size);
    let mut region = MapRegion::rect(size, canvas.map_window());
    // A crawl's worth of excluded pixels.
    for y in 180..230 {
        for x in 16..400 {
            if (x + y) % 3 == 0 {
                region.exclude(x, y);
            }
        }
    }
    let map = step(70, BrightnessScope::Map);
    let whole = step(70, BrightnessScope::Frame);
    let mut frame = PresentedFrame {
        size,
        rgba: pattern(size, 1),
        smooth: false,
        map_region: Some(region),
    };
    let runs = 50;
    let time = |step: &BrightnessStep, frame: &mut PresentedFrame| {
        let start = Instant::now();
        for _ in 0..runs {
            step.dim(Some(MapViewKind::World), frame);
        }
        start.elapsed() / runs
    };
    let map_cost = time(&map, &mut frame);
    let whole_cost = time(&whole, &mut frame);
    eprintln!("[brightness] at 688x240: map {map_cost:?}, whole copy {whole_cost:?}");
    if cfg!(debug_assertions) {
        return;
    }
    assert!(map_cost < Duration::from_millis(1), "{map_cost:?}");
    assert!(whole_cost < Duration::from_millis(1), "{whole_cost:?}");
}

//! The foundation's fresh-save spoiler test for the display chapter
//! (Phase B design 1.4.7, 2.2.9 and 2.6; build plan M1-16).
//!
//! From the factory seed save after character creation, with every display
//! key on at its most revealing value (the wider view with the whole-screen
//! daylight reach, the crawl, typed colour, health colours, the effects,
//! the centred party and the widest named window), the player walks out
//! and opens the docked column with Ctrl+P. On every frame:
//!
//! - every character the crawl and the open window draw (the live rows and
//!   the older rows above them) is on the faithful frame of the same or an
//!   earlier frame, cell for cell;
//! - every name the ribbon draws is on the faithful frame;
//! - no ring cell outside the party's sight is drawn (it is black, as a
//!   hidden core cell is) wherever the crawl's text and the column leave
//!   the map showing, and every lit ring cell joins the core through lit
//!   cells;
//! - and the game's trajectory, frame and key queue are those of Classic:
//!   the cells seen only in the wider margin are never handed to anything
//!   else as seen, because nothing is written at all.
//!
//! Runs only when `U5_TEST_ASSET_DIR` names a writable copy of the game.

use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::path::PathBuf;

use bevy::input::ButtonState;
use bevy::prelude::*;
use bevy::window::WindowResolution;
use u5_runtime::enhancements::{
    BrightnessLut, CliSetting, HostCapabilities, LockState, PixelAspect, ResolveInput, SpriteMasks,
    resolve,
};
use u5_runtime::test_fixtures::{
    configured_original_asset_dir, copy_asset_writable, unique_scratch_dir,
};
use u5_runtime::{
    Area, ChargenStats, FixedCellFont, MapViewKind, ObjectCellPeek, PeekLightPolicy, PeekShade,
    PeekWindow, PlayState, TILE_ATLAS_TILE_PIXELS, TopDownRenderArea, commit_chargen_save,
    load_play_options_from_save,
};

use super::crawl::layer::CrawlLine;
use super::crawl::sync::{CrawlState, cell_shows, glyph_mask};
use super::effects::TextMask;
use super::grow::bands::ribbon_snapshot;
use super::grow::column::ColumnState;
use super::grow::compose::peek_window;
use super::grow::crawl_paint::CrawlPaint;
use super::grow::{COLUMN_CLEAR_X, Canvas, TILE, flash_active};
use crate::enh::VisualLaunchConfig;
use crate::enh::equivalence::{Owner, Scene, Shell, Timeline, check_shell_equivalence};
use crate::enh::present::PresentHost;

const WINDOW: (u32, u32) = (3440, 1440);
const TEXT_COLUMNS: u8 = 40;
const TEXT_ROWS: u8 = 25;

/// Every display key on, each at its most revealing value.
fn everything_on() -> VisualLaunchConfig {
    let on = |key: &str| CliSetting::Enable(key.into());
    let settings = [
        on("display.grow_left"),
        CliSetting::Set {
            key: "display.grow_left_light".into(),
            value: "scaled".into(),
        },
        on("display.crawl"),
        on("display.typed_colour"),
        on("display.ribbon_hp_colours"),
        on("display.dither"),
        on("display.centre_party"),
        on("display.fullscreen_key"),
    ];
    let enhancements = resolve(ResolveInput {
        stored: None,
        cli: &settings,
        lock: LockState::Off,
        caps: HostCapabilities::ALL,
    });
    assert!(enhancements.notes.is_empty(), "{:?}", enhancements.notes);
    VisualLaunchConfig {
        enhancements,
        ..VisualLaunchConfig::classic()
    }
}

/// Whether `cells` (glyph masks, left to right) stand side by side on
/// some text row of `frame`.
fn on_frame(frame: &[u8], cells: &[[u8; 8]]) -> bool {
    if cells.is_empty() {
        return true;
    }
    let width = cells.len() as u8;
    if width > TEXT_COLUMNS {
        return false;
    }
    (0..TEXT_ROWS).any(|row| {
        (0..=TEXT_COLUMNS - width).any(|column| {
            cells
                .iter()
                .enumerate()
                .all(|(at, mask)| cell_shows(frame, column + at as u8, row, mask))
        })
    })
}

/// A crawl or open-window line, without the blank cells it may pad with.
fn line_cells(line: &CrawlLine) -> Vec<[u8; 8]> {
    let cells: Vec<[u8; 8]> = line.cells.iter().map(|cell| cell.mask).collect();
    let first = cells.iter().position(|mask| *mask != [0; 8]);
    let last = cells.iter().rposition(|mask| *mask != [0; 8]);
    match (first, last) {
        (Some(first), Some(last)) => cells[first..=last].to_vec(),
        _ => Vec::new(),
    }
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

fn presented(shell: &Shell) -> Option<(UVec2, Vec<u8>)> {
    let host = shell.app.world().get_resource::<PresentHost>()?;
    let image = shell
        .app
        .world()
        .resource::<Assets<Image>>()
        .get(host.presented()?)?;
    Some((image.size(), image.data.clone()?))
}

/// The ring cells of `window`, from the party, as `(dx, dy)`.
fn offsets(window: PeekWindow) -> impl Iterator<Item = (usize, (isize, isize))> {
    (0..window.cell_count()).filter_map(move |index| Some((index, window.offset_of(index)?)))
}

/// Checks the ring of one frame: hidden cells black where nothing is drawn
/// over the map, and every lit cell joined to the core through lit cells.
/// Returns the number of lit ring cells.
fn check_ring(
    state: &PlayState,
    canvas: &Canvas,
    size: UVec2,
    rgba: &[u8],
    column_open: bool,
    text: &TextMask,
    context: &str,
) -> usize {
    let window = peek_window(canvas);
    let ring = state
        .peek_top_down_ring(window, PeekLightPolicy::Scaled)
        .expect("the plain top-down view has a ring");
    let (x0, y0, x1, y1) = canvas.map_window();
    // While the column is open the crawl's text is inside it, so the map
    // right of it carries nothing but the map. While it is closed, the
    // crawl's glyphs and outlines (`text`) and the prompt's cap in the
    // left border's cell are left out.
    let left = if column_open {
        COLUMN_CLEAR_X
    } else {
        x0 + TILE
    };
    let mut lit = HashSet::new();
    for (index, (dx, dy)) in offsets(window) {
        let Some(cell) = ring.cells[index] else {
            continue;
        };
        if cell.shade == PeekShade::Lit {
            lit.insert((dx, dy));
            continue;
        }
        let cx = canvas.party_x as isize + dx * TILE as isize;
        let cy = canvas.party_y as isize + dy * TILE as isize;
        for y in cy..cy + TILE as isize {
            for x in cx..cx + TILE as isize {
                let (x, y) = (x as u32, y as u32);
                if x < left || x > x1 || y < y0 || y > y1 || text.contains(x, y) {
                    continue;
                }
                let at = ((y * size.x + x) * 4) as usize;
                assert_eq!(
                    &rgba[at..at + 3],
                    &[0, 0, 0],
                    "{context}: hidden ring cell ({dx}, {dy}) is drawn at ({x}, {y})"
                );
            }
        }
    }
    // Sight: the flood starts at the core's edge and walks lit cells.
    let mut queue: VecDeque<(isize, isize)> = lit
        .iter()
        .copied()
        .filter(|&(dx, dy)| {
            (-1..=1).any(|ox| (-1..=1).any(|oy| PeekWindow::is_core(dx + ox, dy + oy)))
        })
        .collect();
    let mut reached: HashSet<(isize, isize)> = queue.iter().copied().collect();
    while let Some((dx, dy)) = queue.pop_front() {
        for ox in -1..=1 {
            for oy in -1..=1 {
                let next = (dx + ox, dy + oy);
                if lit.contains(&next) && reached.insert(next) {
                    queue.push_back(next);
                }
            }
        }
    }
    let island: Vec<_> = lit.difference(&reached).collect();
    assert!(
        island.is_empty(),
        "{context}: lit ring cells out of the party's sight: {island:?}"
    );
    lit.len()
}

fn ctrl_p(timeline: &mut Timeline, frame: usize) {
    for (at, key, state) in [
        (frame, KeyCode::ControlLeft, ButtonState::Pressed),
        (frame, KeyCode::KeyP, ButtonState::Pressed),
        (frame + 1, KeyCode::KeyP, ButtonState::Released),
        (frame + 1, KeyCode::ControlLeft, ButtonState::Released),
    ] {
        timeline.event(at, key, state, false, Owner::Feature);
    }
}

#[test]
fn a_fresh_save_with_every_display_key_on_reveals_nothing() {
    let Some(dir) = configured_original_asset_dir() else {
        return;
    };
    if !dir.join("INIT.GAM").exists() {
        return;
    }
    let game: PathBuf = unique_scratch_dir("display-spoiler");
    for entry in fs::read_dir(&dir).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            copy_asset_writable(&entry.path(), &game.join(entry.file_name())).unwrap();
        }
    }
    // The factory seed save after character creation, with no sidecar.
    commit_chargen_save(
        &game,
        b"Tester",
        true,
        ChargenStats {
            strength: 15,
            dexterity: 15,
            intelligence: 15,
        },
    )
    .unwrap();
    let options = load_play_options_from_save(&game).unwrap();
    let start = PlayState::load_scene(&game, options).unwrap();
    let scene = Scene::from_game_dir(&game).unwrap();
    let font: FixedCellFont = scene.text_font.clone();

    // Out of the door and about, the column opened after five keys and
    // left open, the effect run through, then more steps.
    let keys = [
        KeyCode::ArrowDown,
        KeyCode::ArrowDown,
        KeyCode::ArrowRight,
        KeyCode::ArrowDown,
        KeyCode::ArrowDown,
        KeyCode::ArrowLeft,
        KeyCode::ArrowDown,
        KeyCode::ArrowDown,
        KeyCode::ArrowRight,
        KeyCode::ArrowUp,
        KeyCode::KeyL,
        KeyCode::ArrowDown,
    ];
    let mut timeline = Timeline::new(5 + keys.len() * 20 + 60);
    for (index, key) in keys.iter().enumerate() {
        timeline.game_tap(5 + index * 20, *key);
    }
    ctrl_p(&mut timeline, 92);

    let launch = everything_on();
    let window = |app: &mut App| {
        app.world_mut().spawn(Window {
            resolution: WindowResolution::new(WINDOW.0 as f32, WINDOW.1 as f32),
            ..default()
        });
    };
    let mut shell = Shell::new(start.clone(), &scene, &launch, |_| {});
    window(&mut shell.app);
    let canvas = Canvas::for_window(WINDOW.0, WINDOW.1, PixelAspect::Dos).unwrap();
    let mut shown: Vec<Vec<u8>> = Vec::new();
    let (mut lines_checked, mut ring_frames, mut lit_cells, mut open_frames) = (0, 0, 0, 0);
    for index in 0..timeline.len() {
        shell.step(timeline.frame(index));
        let frame = faithful(&shell);
        let changed = shown.last() != Some(&frame);
        if changed {
            shown.push(frame.clone());
        }
        let context = format!("frame {index}");
        // The crawl and the open window: every line on a frame shown so far.
        if let Some(crawl) = shell.app.world().get_resource::<CrawlState>() {
            let rows = crawl.rows();
            for line in rows.lines.iter().chain(&rows.older) {
                let cells = line_cells(line);
                assert!(
                    shown.iter().rev().any(|frame| on_frame(frame, &cells)),
                    "{context}: {:?} was never on the faithful frame",
                    line.text()
                );
                lines_checked += 1;
            }
        }
        // The ribbon: every member's name is on the faithful frame.
        for member in ribbon_snapshot(&shell.visual().state).members {
            let cells: Vec<[u8; 8]> = member
                .name
                .trim_end()
                .bytes()
                .map(|code| glyph_mask(&font, code))
                .collect();
            assert!(
                shown.iter().rev().any(|frame| on_frame(frame, &cells)),
                "{context}: the ribbon's {:?} was never on the faithful frame",
                member.name
            );
        }
        // The ring, on frames that peeked it afresh and show it plainly.
        let state = &shell.visual().state;
        let view = shell
            .app
            .world()
            .get_resource::<ColumnState>()
            .map(|column| column.view())
            .unwrap_or_default();
        if view.open {
            open_frames += 1;
        }
        if changed
            && view.step.is_none()
            && state.top_down_world_view_active()
            && !flash_active(state)
            && let Some((size, rgba)) = presented(&shell)
            && size == UVec2::new(canvas.w, canvas.h)
        {
            let drawn = canvas.for_column(view, true);
            let rows = shell.app.world().resource::<CrawlState>().rows();
            let text = if view.open {
                TextMask::empty(&drawn)
            } else {
                TextMask::of_crawl(
                    &drawn,
                    CrawlPaint {
                        rows: &rows,
                        crawl: true,
                        typed_colour: true,
                    },
                )
            };
            lit_cells += check_ring(state, &drawn, size, &rgba, view.open, &text, &context);
            if ring_frames == 0 {
                // The check is not vacuous: the map window painted white
                // fails it.
                let lit_all = vec![0xff; rgba.len()];
                let caught = std::panic::catch_unwind(|| {
                    check_ring(state, &drawn, size, &lit_all, view.open, &text, "white")
                });
                assert!(caught.is_err(), "a drawn hidden cell must be caught");
            }
            ring_frames += 1;
        }
    }
    assert!(open_frames > 0, "Ctrl+P opened the column");
    assert!(lines_checked > 0 && ring_frames > 0 && lit_cells > 0);
    eprintln!(
        "fresh save: {} faithful frames, {lines_checked} lines, {ring_frames} ring frames, \
         {lit_cells} lit ring cells checked, column open {open_frames} frames",
        shown.len()
    );

    // Nothing was written: the run equals Classic without the chord.
    let mismatches = check_shell_equivalence(&start, &scene, &timeline, &launch, window);
    assert_eq!(mismatches, []);
    let _ = fs::remove_dir_all(&game);
}

// ---------------------------------------------------------------------------
// Chapter 9 (design 9.6, build plan M2-08).

/// Sprite compositing (with its outline) and the suggested brightness on
/// the map view.
fn light_on() -> VisualLaunchConfig {
    let settings = [
        CliSetting::Enable("display.sprite_composite".into()),
        CliSetting::Set {
            key: "display.brightness".into(),
            value: "70".into(),
        },
    ];
    let enhancements = resolve(ResolveInput {
        stored: None,
        cli: &settings,
        lock: LockState::Off,
        caps: HostCapabilities::ALL,
    });
    assert!(enhancements.notes.is_empty(), "{:?}", enhancements.notes);
    let light = enhancements.display().light;
    assert!(light.composite && light.halo && light.brightness == 70);
    VisualLaunchConfig {
        enhancements,
        ..VisualLaunchConfig::classic()
    }
}

/// A composited object's cell, waiting for the first frame it is empty.
struct Pending {
    /// The frame it was composited on.
    frame: usize,
    /// What the composite drew under it, as palette indices.
    under: [u8; TILE_ATLAS_TILE_PIXELS],
    /// The terrain under it moves with the animation clocks alone, so a
    /// later frame may show another phase of it.
    animated: bool,
}

/// What [`check_light_frame`] counted on one frame.
#[derive(Default)]
struct LightCounts {
    composited_pixels: usize,
    hidden_cells: usize,
}

/// One presented frame of the map view against its faithful frame: every
/// viewport pixel is the faithful pixel through the brightness table,
/// except a background pixel of a composited object, which is black in the
/// faithful frame and shows the terrain the peek put under the object
/// (through the same table). So brightness never lights a black pixel, and
/// a hidden (all-black) cell stays black. Returns the cells that were
/// composited, with what was drawn under them.
fn check_light_frame(
    faithful: &[u8],
    presented: &[u8],
    peek: &ObjectCellPeek,
    lut: &BrightnessLut,
    palette: &[[u8; 3]],
    counts: &mut LightCounts,
    context: &str,
) -> Vec<(u8, u8)> {
    let width = FRAME_SIZE.x as usize;
    let mut composited = Vec::new();
    for row in 0..VIEW_CELLS {
        for col in 0..VIEW_CELLS {
            let object = peek
                .cells
                .iter()
                .find(|cell| usize::from(cell.col) == col && usize::from(cell.row) == row);
            let mut all_black = true;
            let mut drawn_under = false;
            for y in 0..TILE_SIDE {
                for x in 0..TILE_SIDE {
                    let px = VIEW_ORIGIN + col * TILE_SIDE + x;
                    let py = VIEW_ORIGIN + row * TILE_SIDE + y;
                    let at = (py * width + px) * 4;
                    let ours = &presented[at..at + 3];
                    let theirs = &faithful[at..at + 3];
                    let dimmed = [
                        lut.apply(theirs[0]),
                        lut.apply(theirs[1]),
                        lut.apply(theirs[2]),
                    ];
                    all_black &= theirs == [0, 0, 0];
                    if ours == dimmed {
                        continue;
                    }
                    let cell = object.unwrap_or_else(|| {
                        panic!(
                            "{context}: cell ({col}, {row}) holds no composited object, yet \
                             ({px}, {py}) is {ours:?}, not {dimmed:?}"
                        )
                    });
                    assert_eq!(
                        theirs,
                        [0, 0, 0],
                        "{context}: a sprite pixel changed at ({px}, {py})"
                    );
                    let terrain = palette[usize::from(cell.under_pixels[y * TILE_SIDE + x])];
                    let terrain = [
                        lut.apply(terrain[0]),
                        lut.apply(terrain[1]),
                        lut.apply(terrain[2]),
                    ];
                    assert_eq!(
                        ours, terrain,
                        "{context}: ({px}, {py}) is not the terrain under the object"
                    );
                    counts.composited_pixels += 1;
                    drawn_under = true;
                }
            }
            if all_black && object.is_none() {
                counts.hidden_cells += 1;
            }
            if drawn_under {
                composited.push((col as u8, row as u8));
            }
        }
    }
    composited
}

const VIEW_CELLS: usize = 11;
const VIEW_ORIGIN: usize = 8;
const TILE_SIDE: usize = 16;
const FRAME_SIZE: UVec2 = UVec2::new(
    crate::VISUAL_PLAY_FRAME_WIDTH,
    crate::VISUAL_PLAY_FRAME_HEIGHT,
);

/// The design's spoiler test for chapter 9 (9.6): from the fresh save with
/// compositing and brightness on, over a scripted walk,
///
/// - the terrain drawn under every composited object is the tile the
///   faithful frame shows in that cell on the first frame the cell is
///   empty (a cell whose terrain animates is matched on its tile, any
///   phase, and counted apart);
/// - brightness and compositing add no non-black pixel to a pixel the
///   faithful frame leaves black, outside an object's own background: a
///   hidden cell stays black;
/// - and the run equals Classic.
///
/// Runs only when `U5_TEST_ASSET_DIR` names a writable copy of the game.
#[test]
fn a_fresh_save_with_compositing_and_brightness_reveals_nothing() {
    let Some(dir) = configured_original_asset_dir() else {
        return;
    };
    if !dir.join("INIT.GAM").exists() {
        return;
    }
    let game: PathBuf = unique_scratch_dir("light-spoiler");
    for entry in fs::read_dir(&dir).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            copy_asset_writable(&entry.path(), &game.join(entry.file_name())).unwrap();
        }
    }
    commit_chargen_save(
        &game,
        b"Tester",
        true,
        ChargenStats {
            strength: 15,
            dexterity: 15,
            intelligence: 15,
        },
    )
    .unwrap();
    let options = load_play_options_from_save(&game).unwrap();
    let start = PlayState::load_scene(&game, options).unwrap();
    let scene = Scene::from_game_dir(&game).unwrap();
    let masks = SpriteMasks::from_atlas(&scene.atlas);
    let palette = crate::visual_palette(scene.atlas.depth);
    let lut = BrightnessLut::for_percent(70);

    // Out of the door and about, and back, so cells the party and the
    // people stood on come back into view empty.
    let keys = [
        KeyCode::ArrowDown,
        KeyCode::ArrowDown,
        KeyCode::ArrowRight,
        KeyCode::ArrowDown,
        KeyCode::ArrowDown,
        KeyCode::ArrowLeft,
        KeyCode::ArrowDown,
        KeyCode::ArrowDown,
        KeyCode::ArrowRight,
        KeyCode::ArrowUp,
        KeyCode::ArrowUp,
        KeyCode::ArrowLeft,
        KeyCode::ArrowUp,
        KeyCode::Space,
        KeyCode::ArrowDown,
    ];
    let mut timeline = Timeline::new(5 + keys.len() * 20 + 60);
    for (index, key) in keys.iter().enumerate() {
        timeline.game_tap(5 + index * 20, *key);
    }
    let launch = light_on();
    let mut shell = Shell::new(start.clone(), &scene, &launch, |_| {});
    let mut pending: HashMap<(Area, Option<i8>, usize, usize), Pending> = HashMap::new();
    let mut counts = LightCounts::default();
    let (mut map_frames, mut matched, mut matched_animated, mut dark_when_empty) = (0, 0, 0, 0);
    let mut last = None;
    for index in 0..timeline.len() {
        shell.step(timeline.frame(index));
        let state = &shell.visual().state;
        let kind = state.map_view_kind();
        let Some(area) = state.top_down_render_area() else {
            continue;
        };
        if !kind.is_some_and(MapViewKind::is_top_down) || flash_active(state) {
            continue;
        }
        let frame = faithful(&shell);
        let Some((size, rgba)) = presented(&shell) else {
            continue;
        };
        assert_eq!(size, FRAME_SIZE, "frame {index}");
        if last.as_ref() == Some(&(frame.clone(), rgba.clone())) {
            continue;
        }
        let context = format!("frame {index}");
        let peek = state
            .peek_object_cells(&scene.atlas, &masks)
            .unwrap_or_default();
        let composited =
            check_light_frame(&frame, &rgba, &peek, &lut, palette, &mut counts, &context);
        map_frames += 1;
        // The cell's world coordinate, as the peek reads it.
        let wrap = matches!(area, TopDownRenderArea::World(_));
        let floor = state.current_floor();
        let at = |col: usize, row: usize| -> Option<(usize, usize)> {
            let x = state.player.x as isize + col as isize - 5;
            let y = state.player.y as isize + row as isize - 5;
            if wrap {
                Some((x.rem_euclid(256) as usize, y.rem_euclid(256) as usize))
            } else {
                (x >= 0 && y >= 0).then_some((x as usize, y as usize))
            }
        };
        // Cells composited earlier that are empty now: no object on them
        // and not the party's cell.
        let occupied = |x: usize, y: usize| {
            (x, y) == (state.player.x, state.player.y)
                || state.active_objects.iter().any(|object| {
                    !object.is_empty() && object.x == x && object.y == y && Some(object.z) == floor
                })
        };
        for row in 0..VIEW_CELLS {
            for col in 0..VIEW_CELLS {
                let Some((x, y)) = at(col, row) else {
                    continue;
                };
                let key = (state.area, floor, x, y);
                if occupied(x, y) || !pending.contains_key(&key) {
                    continue;
                }
                let waiting = pending.remove(&key).unwrap();
                let block: Vec<[u8; 3]> = (0..TILE_SIDE * TILE_SIDE)
                    .map(|i| {
                        let px = VIEW_ORIGIN + col * TILE_SIDE + i % TILE_SIDE;
                        let py = VIEW_ORIGIN + row * TILE_SIDE + i / TILE_SIDE;
                        let at = (py * FRAME_SIZE.x as usize + px) * 4;
                        [frame[at], frame[at + 1], frame[at + 2]]
                    })
                    .collect();
                if block.iter().all(|px| *px == [0, 0, 0]) {
                    // Out of sight now: nothing to compare with, and the
                    // presented cell was checked black above.
                    dark_when_empty += 1;
                    continue;
                }
                let drawn: Vec<[u8; 3]> = waiting
                    .under
                    .iter()
                    .map(|&index| palette[usize::from(index)])
                    .collect();
                if block == drawn {
                    matched += 1;
                } else {
                    assert!(
                        waiting.animated,
                        "{context}: the terrain drawn under the object at ({x}, {y}) on frame \
                         {} is not the tile the cell shows empty",
                        waiting.frame
                    );
                    matched_animated += 1;
                }
            }
        }
        // This frame's composited cells wait for their first empty frame.
        for (col, row) in composited {
            let Some((x, y)) = at(usize::from(col), usize::from(row)) else {
                continue;
            };
            let cell = peek
                .cells
                .iter()
                .find(|cell| (cell.col, cell.row) == (col, row))
                .unwrap();
            pending.entry((state.area, floor, x, y)).or_insert(Pending {
                frame: index,
                under: cell.under_pixels,
                animated: peek.animated_under,
            });
        }
        last = Some((frame, rgba));
    }
    eprintln!(
        "fresh save, chapter 9: {map_frames} map frames, {} composited pixels, {} hidden cells \
         checked black; {matched} cells matched empty, {matched_animated} animated, \
         {dark_when_empty} dark when empty, {} never seen empty",
        counts.composited_pixels,
        counts.hidden_cells,
        pending.len()
    );
    assert!(map_frames > 0 && counts.composited_pixels > 0 && counts.hidden_cells > 0);
    assert!(matched > 0, "no composited cell was seen empty");

    let mismatches = check_shell_equivalence(&start, &scene, &timeline, &launch, |_| {});
    assert_eq!(mismatches, []);
    let _ = fs::remove_dir_all(&game);
}

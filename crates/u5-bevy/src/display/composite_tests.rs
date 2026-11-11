//! The `[composite]` step (design 9.3.8; build plan M2-06): only black mask
//! pixels of object cells change, the listed and near-empty tiles keep
//! their box, a flash and anything painted over a sprite are kept, water
//! under a ship repaints on its own clock, the halo on and off, the
//! grow-left core carries the composite, the menu's changes, and with the
//! key off no step and no masks. M2-07 (P5b, 9.4.4): a sprite in the ring
//! equals the same sprite on the same terrain in the core, the ring flash
//! comes after the composite, and with the key off the ring is boxed.

use bevy::window::WindowResolution;
use u5_runtime::enhancements::{
    CliSetting, DARK_BODY, HostCapabilities, KEEP_THE_BOX, LockState, ResolveInput, mask_count,
    resolve,
};
use u5_runtime::test_fixtures::{
    open_grid, open_world_grid, synthetic_tile_atlas, test_state, world_state,
};
use u5_runtime::{
    ACTOR_TILE_BANK_BASE, ActiveObject, FULL_DAYLIGHT, ObjectCell, TILE_ATLAS_TILE_COUNT,
    TILE_ATLAS_TILE_PIXELS, TileAtlas, TileGraphicsDepth, TransportState, VIEWPORT_PLAYER_ROW,
    hash_bytes, water_pass_rotates_tile,
};

use super::*;
use crate::VisualState;
use crate::display::grow::column::ColumnView;
use crate::display::grow::{Canvas, GrowLeftCompose, compose};
use crate::enh::VisualLaunchConfig;
use crate::enh::equivalence::{Scene, Shell, Timeline, check_shell_equivalence};

/// An ordinary townsperson.
const PERSON: u8 = 0x5A;
/// The frigate.
const FRIGATE: u8 = 168;
/// The actor byte whose tile the boxed atlas leaves near-empty.
const SPECK: u8 = 0x0A;
/// A combat field (atlas 0x1E8).
const FIELD: u8 = 0xE8;

/// The synthetic atlas with every actor-half tile drawn as an 8x8 coloured
/// square (pixels 4..12) on a black box, except [`SPECK`]'s, a 3x3 square:
/// a near-empty tile. Terrain tiles are solid and never black, so the
/// terrain shown through a box is visible.
fn boxed_atlas(depth: TileGraphicsDepth) -> TileAtlas {
    let mut atlas = synthetic_tile_atlas(depth);
    let limit = depth.pixel_limit();
    for tile in 0..ACTOR_TILE_BANK_BASE {
        let colour = (tile % (usize::from(limit) - 1)) as u8 + 1;
        let base = tile * TILE_ATLAS_TILE_PIXELS;
        atlas.pixels[base..base + TILE_ATLAS_TILE_PIXELS].fill(colour);
    }
    for tile in ACTOR_TILE_BANK_BASE..TILE_ATLAS_TILE_COUNT {
        let colour = (tile % (usize::from(limit) - 1)) as u8 + 1;
        let side = if tile == ACTOR_TILE_BANK_BASE + usize::from(SPECK) {
            7..10
        } else {
            4..12
        };
        let base = tile * TILE_ATLAS_TILE_PIXELS;
        for at in 0..TILE_ATLAS_TILE_PIXELS {
            let (x, y) = (at % 16, at / 16);
            let inside = side.contains(&x) && side.contains(&y);
            atlas.pixels[base + at] = if inside { colour } else { 0 };
        }
    }
    atlas
}

fn boxed_scene() -> Scene {
    Scene {
        atlas: boxed_atlas(TileGraphicsDepth::Ega16),
        ..Scene::synthetic()
    }
}

fn object(tile: u8, x: usize, y: usize) -> ActiveObject {
    ActiveObject {
        type_byte: tile,
        tile,
        x,
        y,
        z: 0,
        ..ActiveObject::empty()
    }
}

/// A town at noon with the party at (5, 5) and `objects` around it.
fn town_with(objects: &[ActiveObject]) -> PlayState {
    let mut state = test_state(open_grid(), 5, 5);
    state.ambient_light = FULL_DAYLIGHT;
    state.active_objects.truncate(1);
    state.active_objects.extend_from_slice(objects);
    state
}

/// Two people, a listed tile of each list, the near-empty tile and a
/// field, all in view.
fn busy_town() -> PlayState {
    town_with(&[
        object(PERSON, 3, 4),
        object(PERSON + 1, 7, 6),
        object((KEEP_THE_BOX[0] - ACTOR_TILE_BANK_BASE) as u8, 2, 2),
        object((DARK_BODY[1] - ACTOR_TILE_BANK_BASE) as u8, 8, 2),
        object(SPECK, 2, 8),
        object(FIELD, 8, 8),
    ])
}

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

fn on() -> CliSetting {
    CliSetting::Enable("display.sprite_composite".into())
}

fn set(key: &str, value: &str) -> CliSetting {
    CliSetting::Set {
        key: key.into(),
        value: value.into(),
    }
}

fn shell_with(state: PlayState, launch: &VisualLaunchConfig, width: f32, height: f32) -> Shell {
    let mut shell = Shell::new(state, &boxed_scene(), launch, |_| {});
    shell.app.world_mut().spawn(Window {
        resolution: WindowResolution::new(width, height),
        ..default()
    });
    shell
}

fn orders(shell: &Shell) -> Option<Vec<StepOrder>> {
    shell
        .app
        .world()
        .get_resource::<PresentSteps>()
        .map(PresentSteps::orders)
}

/// The faithful frame's bytes, the frame sprite's image's bytes and size,
/// and whether the sprite shows the faithful handle.
fn images(shell: &mut Shell) -> (Vec<u8>, Vec<u8>, UVec2, bool) {
    let world = shell.app.world_mut();
    let faithful = world.resource::<VisualState>().image_handle.clone();
    let mut sprites = world.query::<&Sprite>();
    let shown = sprites.single(world).unwrap().image.clone();
    let assets = world.resource::<Assets<Image>>();
    let bytes = |handle: &Handle<Image>| assets.get(handle).unwrap().data.clone().unwrap();
    let size = assets.get(&shown).unwrap().size();
    (bytes(&faithful), bytes(&shown), size, shown == faithful)
}

/// What the step should show for the shell's current frame: the faithful
/// bytes composited from a fresh peek of the shell's state.
fn expected_composite(shell: &Shell, faithful: &[u8], halo: bool) -> (Vec<u8>, usize) {
    let visual = shell.app.world().resource::<VisualState>();
    let masks = SpriteMasks::from_atlas(&visual.atlas);
    let mut rgba = faithful.to_vec();
    let changed = visual
        .state
        .peek_object_cells(&visual.atlas, &masks)
        .map_or(0, |peek| {
            composite_cells(
                &mut rgba,
                crate::visual_palette(visual.atlas.depth),
                &masks,
                &peek,
                halo,
            )
        });
    (rgba, changed)
}

/// A walk on the open grid.
fn walk(frames: usize) -> Timeline {
    let mut timeline = Timeline::new(frames);
    timeline
        .game_tap(5, KeyCode::ArrowRight)
        .game_tap(15, KeyCode::ArrowDown)
        .game_tap(25, KeyCode::ArrowLeft);
    timeline
}

fn pixel(rgba: &[u8], x: usize, y: usize) -> [u8; 3] {
    let at = (y * FRAME.x as usize + x) * 4;
    [rgba[at], rgba[at + 1], rgba[at + 2]]
}

/// The 16x16 cell at viewport `(col, row)` of a 320x200 frame.
fn cell_pixels(rgba: &[u8], col: usize, row: usize) -> Vec<[u8; 3]> {
    (0..TILE_ATLAS_TILE_PIXELS)
        .map(|i| {
            pixel(
                rgba,
                VIEWPORT_ORIGIN + col * 16 + i % 16,
                VIEWPORT_ORIGIN + row * 16 + i / 16,
            )
        })
        .collect()
}

/// A cell of `drawn` at `(col, row)` over terrain of palette `under`.
fn cell(col: u8, row: u8, drawn: u16, under: u8) -> ObjectCell {
    ObjectCell {
        col,
        row,
        drawn,
        under: 0,
        under_pixels: [under; TILE_ATLAS_TILE_PIXELS],
    }
}

/// A frame of every palette colour, with each sprite of `cells` drawn as
/// the atlas has it, so its box is black.
fn frame_with_sprites(atlas: &TileAtlas, cells: &[ObjectCell]) -> Vec<u8> {
    let palette = crate::visual_palette(atlas.depth);
    let mut rgba: Vec<u8> = (0..(FRAME.x * FRAME.y) as usize)
        .flat_map(|i| {
            let [r, g, b] = palette[(i * 7 + i / 320) % palette.len()];
            [r, g, b, 0xff]
        })
        .collect();
    for cell in cells {
        let tile = atlas.tile_pixels(usize::from(cell.drawn)).unwrap();
        for (i, index) in tile.iter().enumerate() {
            let x = VIEWPORT_ORIGIN + usize::from(cell.col) * 16 + i % 16;
            let y = VIEWPORT_ORIGIN + usize::from(cell.row) * 16 + i / 16;
            let at = (y * FRAME.x as usize + x) * 4;
            rgba[at..at + 3].copy_from_slice(&palette[usize::from(*index)]);
        }
    }
    rgba
}

// --- The pixel rule (9.3.8 steps 2 and 3). ----------------------------------

#[test]
fn only_black_mask_pixels_of_object_cells_change() {
    for depth in [TileGraphicsDepth::Ega16, TileGraphicsDepth::Cga4] {
        let atlas = boxed_atlas(depth);
        let masks = SpriteMasks::from_atlas(&atlas);
        let palette = crate::visual_palette(depth);
        let under = (palette.len() - 1) as u8;
        let cells = [
            cell(0, 0, 300, under),
            cell(4, 7, 333, under),
            cell(10, 10, 280, under),
        ];
        let mut before = frame_with_sprites(&atlas, &cells);
        // Something already drawn over a background pixel is kept.
        let painted = (VIEWPORT_ORIGIN + 64, VIEWPORT_ORIGIN + 7 * 16);
        let at = (painted.1 * FRAME.x as usize + painted.0) * 4;
        before[at..at + 3].copy_from_slice(&palette[1]);
        for halo in [false, true] {
            let peek = ObjectCellPeek {
                cells: cells.to_vec(),
                animated_under: false,
            };
            let mut after = before.clone();
            let changed = composite_cells(&mut after, palette, &masks, &peek, halo);
            let mut counted = 0;
            for y in 0..FRAME.y as usize {
                for x in 0..FRAME.x as usize {
                    let in_mask = cells.iter().any(|cell| {
                        let (cx, cy) = (
                            VIEWPORT_ORIGIN + usize::from(cell.col) * 16,
                            VIEWPORT_ORIGIN + usize::from(cell.row) * 16,
                        );
                        (cx..cx + 16).contains(&x)
                            && (cy..cy + 16).contains(&y)
                            && mask_bit(
                                masks.background(usize::from(cell.drawn), halo),
                                x - cx,
                                y - cy,
                            )
                    });
                    let was_black = pixel(&before, x, y) == palette[0];
                    let expected = if in_mask && was_black {
                        counted += 1;
                        palette[usize::from(under)]
                    } else {
                        pixel(&before, x, y)
                    };
                    assert_eq!(
                        pixel(&after, x, y),
                        expected,
                        "{depth:?} halo {halo} ({x}, {y})"
                    );
                }
            }
            // Alpha is never touched.
            assert!(after.chunks(4).all(|px| px[3] == 0xff));
            assert_eq!(changed, counted);
            // The 8x8 sprite leaves 192 background pixels, one of them
            // painted over; the halo keeps the 36 round the square.
            let per_cell = if halo { 192 - 36 } else { 192 };
            assert_eq!(changed, per_cell * 3 - 1, "{depth:?} halo {halo}");
        }
    }
}

#[test]
fn the_halo_keeps_a_one_pixel_black_outline() {
    let atlas = boxed_atlas(TileGraphicsDepth::Ega16);
    let masks = SpriteMasks::from_atlas(&atlas);
    let palette = crate::visual_palette(atlas.depth);
    let cells = [cell(5, 5, 300, 2)];
    let before = frame_with_sprites(&atlas, &cells);
    let peek = ObjectCellPeek {
        cells: cells.to_vec(),
        animated_under: false,
    };
    let (mut with, mut without) = (before.clone(), before.clone());
    composite_cells(&mut with, palette, &masks, &peek, true);
    composite_cells(&mut without, palette, &masks, &peek, false);
    let (with, without) = (cell_pixels(&with, 5, 5), cell_pixels(&without, 5, 5));
    for (i, (h, f)) in with.iter().zip(&without).enumerate() {
        let (x, y) = (i % 16, i / 16);
        let sprite = (4..12).contains(&x) && (4..12).contains(&y);
        let outline = !sprite && (3..13).contains(&x) && (3..13).contains(&y);
        if sprite {
            assert_eq!(h, f, "({x}, {y}) is the sprite");
            assert_ne!(*h, palette[0]);
        } else if outline {
            // Corners included: the 8-neighbour halo closes them.
            assert_eq!(*h, palette[0], "({x}, {y}) is the halo");
            assert_eq!(*f, palette[2], "({x}, {y}) without the halo");
        } else {
            assert_eq!((*h, *f), (palette[2], palette[2]), "({x}, {y})");
        }
    }
    assert_eq!(mask_count(masks.background(300, false)), 192);
    assert_eq!(mask_count(masks.background(300, true)), 156);
}

/// A flash inverts the view after the render: no background pixel is black
/// then, so the frame shows as with the step off (9.2.5).
#[test]
fn a_flash_looks_exactly_as_with_the_step_off() {
    let atlas = boxed_atlas(TileGraphicsDepth::Ega16);
    let masks = SpriteMasks::from_atlas(&atlas);
    let palette = crate::visual_palette(atlas.depth);
    let cells = [cell(2, 3, 300, 2), cell(5, 5, 301, 9)];
    let mut flashed = frame_with_sprites(&atlas, &cells);
    let playback = u5_runtime::potion_flash_playback(0).expect("the flash playback");
    assert!(crate::apply_visual_potion_flash_xor(&mut flashed, playback));
    let peek = ObjectCellPeek {
        cells: cells.to_vec(),
        animated_under: false,
    };
    for halo in [true, false] {
        let mut after = flashed.clone();
        assert_eq!(composite_cells(&mut after, palette, &masks, &peek, halo), 0);
        assert_eq!(after, flashed);
    }
}

/// The combat cursor and aim marker are painted over the finished
/// viewport: where they cover a background pixel it is not black, so it is
/// kept (9.2.5). (The peek leaves the marker's cell out altogether, M2-05.)
#[test]
fn the_cursor_and_marker_pixels_are_kept() {
    let atlas = boxed_atlas(TileGraphicsDepth::Ega16);
    let masks = SpriteMasks::from_atlas(&atlas);
    let palette = crate::visual_palette(atlas.depth);
    let cells = [cell(4, 4, 300, 2)];
    let plain = frame_with_sprites(&atlas, &cells);
    // A white ring on the cell's two outer pixel rings, as the cursor.
    let mut cursor = plain.clone();
    let ring = [0, 1, 14, 15];
    for i in 0..TILE_ATLAS_TILE_PIXELS {
        let (x, y) = (i % 16, i / 16);
        if ring.contains(&x) || ring.contains(&y) {
            let at = ((VIEWPORT_ORIGIN + 64 + y) * FRAME.x as usize + VIEWPORT_ORIGIN + 64 + x) * 4;
            cursor[at..at + 3].copy_from_slice(&palette[15]);
        }
    }
    let peek = ObjectCellPeek {
        cells: cells.to_vec(),
        animated_under: false,
    };
    let (mut a, mut b) = (plain.clone(), cursor.clone());
    composite_cells(&mut a, palette, &masks, &peek, true);
    composite_cells(&mut b, palette, &masks, &peek, true);
    for i in 0..TILE_ATLAS_TILE_PIXELS {
        let (x, y) = (i % 16, i / 16);
        let (px, py) = (VIEWPORT_ORIGIN + 64 + x, VIEWPORT_ORIGIN + 64 + y);
        if ring.contains(&x) || ring.contains(&y) {
            assert_eq!(pixel(&b, px, py), palette[15], "({x}, {y}) is the cursor");
        } else {
            assert_eq!(pixel(&b, px, py), pixel(&a, px, py), "({x}, {y})");
        }
    }
}

// --- The step's peek and revision (9.3.8 steps 1 and 4). ---------------------

fn input_for<'a>(
    faithful: &'a [u8],
    hash: u64,
    state: Option<&'a PlayState>,
    atlas: Option<&'a TileAtlas>,
) -> PresentInput<'a> {
    PresentInput {
        faithful,
        size: FRAME,
        faithful_hash: hash,
        state,
        atlas,
        font: None,
        window: UVec2::ZERO,
        scale_factor: 1.0,
        side: None,
    }
}

/// The world with the party on a frigate on water, at noon, rendered once
/// so the peek has buffers to read. The water tile gets a row pattern, so
/// the scroll moves its pixels.
fn ship_on_water() -> (PlayState, TileAtlas, u8) {
    let mut atlas = boxed_atlas(TileGraphicsDepth::Ega16);
    let water = (0..=u8::MAX)
        .find(|&tile| water_pass_rotates_tile(tile))
        .expect("a rotated water id");
    let base = usize::from(water) * TILE_ATLAS_TILE_PIXELS;
    for at in 0..TILE_ATLAS_TILE_PIXELS {
        atlas.pixels[base + at] = if (at / 16) % 4 == 0 { 15 } else { 1 };
    }
    let mut state = world_state(vec![water; open_world_grid().len()], 100, 100);
    state.ambient_light = FULL_DAYLIGHT;
    state.player.transport = TransportState::Ship {
        type_byte: FRIGATE,
        tile: FRIGATE,
        sails_hoisted: false,
        hull: 100,
        skiffs: 1,
    };
    state
        .render_top_down_base_frame(VIEWPORT_PLAYER_ROW, &atlas)
        .unwrap();
    (state, atlas, water)
}

#[test]
fn water_under_a_ship_repaints_on_a_scroll_change_with_the_same_frame() {
    let (mut state, atlas, _) = ship_on_water();
    let faithful = vec![0u8; (FRAME.x * FRAME.y * 4) as usize];
    let mut step = CompositeStep::new(Arc::default(), true);
    let first = step.revision(&input_for(&faithful, 7, Some(&state), Some(&atlas)));
    assert_eq!(step.peeks(), 1);
    let peek = step.cache.as_ref().unwrap().peek.clone().unwrap();
    assert!(peek.animated_under);
    let ship = peek
        .cells
        .iter()
        .find(|cell| (cell.col, cell.row) == (5, 5))
        .unwrap();
    // The same frame and clocks: the cached peek.
    assert_eq!(
        step.revision(&input_for(&faithful, 7, Some(&state), Some(&atlas))),
        first
    );
    assert_eq!(step.peeks(), 1);
    // The water scrolls; the faithful frame is unchanged.
    state.water_scroll.tick();
    let second = step.revision(&input_for(&faithful, 7, Some(&state), Some(&atlas)));
    assert_eq!(step.peeks(), 2);
    assert_ne!(second, first);
    let moved = step.cache.as_ref().unwrap().peek.clone().unwrap();
    let ship_after = moved
        .cells
        .iter()
        .find(|cell| (cell.col, cell.row) == (5, 5))
        .unwrap();
    assert_ne!(ship_after.under_pixels, ship.under_pixels);
    // And the composited copy shows the new water.
    let mut frame = PresentedFrame {
        size: FRAME,
        rgba: faithful.clone(),
        ..PresentedFrame::default()
    };
    step.apply(
        &input_for(&faithful, 7, Some(&state), Some(&atlas)),
        &mut frame,
    );
    let masks = SpriteMasks::from_atlas(&atlas);
    let mut expected = faithful.clone();
    composite_cells(
        &mut expected,
        crate::visual_palette(atlas.depth),
        &masks,
        &moved,
        true,
    );
    assert_eq!(frame.rgba, expected);
    assert_ne!(frame.rgba, faithful);
}

#[test]
fn still_terrain_is_not_peeked_again_on_an_animation_tick() {
    let atlas = boxed_atlas(TileGraphicsDepth::Ega16);
    let mut state = town_with(&[object(PERSON, 3, 4)]);
    state
        .render_top_down_base_frame(VIEWPORT_PLAYER_ROW, &atlas)
        .unwrap();
    let faithful = vec![0u8; (FRAME.x * FRAME.y * 4) as usize];
    let mut step = CompositeStep::new(Arc::default(), true);
    let first = step.revision(&input_for(&faithful, 1, Some(&state), Some(&atlas)));
    let peek = step.cache.as_ref().unwrap().peek.clone().unwrap();
    assert!(!peek.animated_under);
    assert_eq!(peek.cells.len(), 2, "the party and the person: {peek:?}");
    state.water_scroll.tick();
    state.animation.frame = state.animation.frame.wrapping_add(1);
    assert_eq!(
        step.revision(&input_for(&faithful, 1, Some(&state), Some(&atlas))),
        first
    );
    assert_eq!(step.peeks(), 1);
    // A new faithful frame peeks again.
    step.revision(&input_for(&faithful, 2, Some(&state), Some(&atlas)));
    assert_eq!(step.peeks(), 2);
}

#[test]
fn the_revision_follows_the_cells_and_the_halo() {
    let atlas = boxed_atlas(TileGraphicsDepth::Ega16);
    let faithful = vec![0u8; (FRAME.x * FRAME.y * 4) as usize];
    let mut revisions = std::collections::HashSet::new();
    for objects in [
        vec![object(PERSON, 3, 4)],
        vec![object(PERSON, 4, 4)],
        vec![object(PERSON + 1, 3, 4)],
    ] {
        let mut state = town_with(&objects);
        state
            .render_top_down_base_frame(VIEWPORT_PLAYER_ROW, &atlas)
            .unwrap();
        for halo in [true, false] {
            let mut step = CompositeStep::new(Arc::default(), halo);
            let revision = step.revision(&input_for(&faithful, 1, Some(&state), Some(&atlas)));
            assert!(revisions.insert(revision), "{objects:?} halo {halo}");
        }
    }
    // Before play (the intro) there is nothing to peek and no masks yet.
    let mut step = CompositeStep::new(Arc::default(), true);
    step.revision(&input_for(&faithful, 1, None, None));
    assert_eq!(step.peeks(), 0);
    assert!(step.masks.get().is_none());
}

// --- In the shell. -----------------------------------------------------------

#[test]
fn off_there_is_no_step_and_no_masks() {
    for cli in [
        vec![],
        vec![set("display.composite_halo", "false")],
        vec![CliSetting::Disable("display.sprite_composite".into())],
    ] {
        let launch = launch(&cli);
        assert!(launch.enhancements.notes.is_empty(), "{cli:?}");
        let mut shell = shell_with(busy_town(), &launch, 1280.0, 960.0);
        for _ in 0..3 {
            shell.step(&[]);
        }
        assert_eq!(orders(&shell), None, "{cli:?}");
        assert!(!shell.app.world().contains_resource::<CompositeMasks>());
        let (faithful, shown, _, faithful_handle) = images(&mut shell);
        assert!(
            faithful_handle,
            "{cli:?}: the sprite shows the faithful image"
        );
        assert_eq!(faithful, shown);
    }
}

#[test]
fn on_the_step_comes_first_with_its_masks() {
    let mut shell = shell_with(busy_town(), &launch(&[on()]), 1280.0, 960.0);
    assert_eq!(orders(&shell), Some(vec![StepOrder::COMPOSITE]));
    let masks = shell.app.world().resource::<CompositeMasks>().clone();
    assert!(masks.get().is_none(), "built on the first play frame");
    shell.step(&[]);
    assert!(masks.get().is_some());
    let all = launch(&[
        on(),
        set("display.brightness", "70"),
        CliSetting::Enable("display.grow_left".into()),
        set("display.scale_mode", "sharp"),
    ]);
    assert_eq!(
        orders(&shell_with(busy_town(), &all, 1920.0, 1080.0)),
        Some(vec![
            StepOrder::COMPOSITE,
            StepOrder::GROW_LEFT_COMPOSE,
            StepOrder::BRIGHTNESS,
            StepOrder::PRESCALE
        ])
    );
}

#[test]
fn the_shell_composites_the_object_cells_and_leaves_the_faithful_image() {
    let mut off = shell_with(busy_town(), &VisualLaunchConfig::classic(), 1280.0, 960.0);
    let mut on = shell_with(busy_town(), &launch(&[on()]), 1280.0, 960.0);
    let timeline = walk(40);
    let mut composited = 0;
    for index in 0..timeline.len() {
        off.step(timeline.frame(index));
        on.step(timeline.frame(index));
        let (faithful_off, _, _, _) = images(&mut off);
        let (faithful_on, shown, size, faithful_handle) = images(&mut on);
        assert_eq!(faithful_on, faithful_off, "frame {index}: faithful bytes");
        assert_eq!(hash_bytes(&faithful_on), hash_bytes(&faithful_off));
        assert_eq!(size, FRAME);
        assert!(!faithful_handle);
        let (expected, changed) = expected_composite(&on, &faithful_on, true);
        assert_eq!(shown, expected, "frame {index}");
        assert_eq!(changed > 0, shown != faithful_on, "frame {index}");
        composited += changed;
    }
    assert!(composited > 0);
}

#[test]
fn listed_and_near_empty_tiles_keep_their_box() {
    let mut shell = shell_with(busy_town(), &launch(&[on()]), 1280.0, 960.0);
    let boxed: Vec<u16> = [
        KEEP_THE_BOX[0],
        DARK_BODY[1],
        ACTOR_TILE_BANK_BASE + usize::from(SPECK),
        ACTOR_TILE_BANK_BASE + usize::from(FIELD),
    ]
    .iter()
    .map(|&tile| tile as u16)
    .collect();
    let (mut kept, mut people) = (std::collections::BTreeSet::new(), 0);
    for _ in 0..10 {
        shell.step(&[]);
        let (faithful, shown, _, _) = images(&mut shell);
        let visual = shell.app.world().resource::<VisualState>();
        let state = &visual.state;
        let (px, py) = (state.player.x as isize, state.player.y as isize);
        for object in state.active_objects.iter().skip(1) {
            let col = object.x as isize - px + 5;
            let row = object.y as isize - py + 5;
            if !(0..11).contains(&col) || !(0..11).contains(&row) {
                continue;
            }
            let drawn = u16::from(object.tile) + ACTOR_TILE_BANK_BASE as u16;
            let (col, row) = (col as usize, row as usize);
            let same = cell_pixels(&faithful, col, row) == cell_pixels(&shown, col, row);
            if boxed.contains(&drawn) {
                assert!(same, "tile {drawn} at ({col}, {row}) keeps its box");
                kept.insert(drawn);
            } else if !same {
                people += 1;
            }
        }
    }
    assert_eq!(
        kept.len(),
        boxed.len(),
        "every boxed object was in view: {kept:?}"
    );
    assert!(people > 0, "the people composite");
}

#[test]
fn halo_on_and_off_in_the_shell() {
    for halo in [true, false] {
        let mut cli = vec![on()];
        if !halo {
            cli.push(set("display.composite_halo", "false"));
        }
        let mut shell = shell_with(busy_town(), &launch(&cli), 1280.0, 960.0);
        shell.step(&[]);
        shell.step(&[]);
        let (faithful, shown, _, _) = images(&mut shell);
        let (expected, changed) = expected_composite(&shell, &faithful, halo);
        assert_eq!(shown, expected, "halo {halo}");
        // The party and two people at 156 or 192 pixels each.
        assert_eq!(changed, 3 * if halo { 156 } else { 192 }, "halo {halo}");
    }
}

#[test]
fn the_game_runs_the_same_with_the_step_on() {
    for cli in [
        vec![on()],
        vec![on(), set("display.composite_halo", "false")],
    ] {
        let mismatches = check_shell_equivalence(
            &busy_town(),
            &boxed_scene(),
            &walk(40),
            &launch(&cli),
            |app| {
                app.world_mut().spawn(Window {
                    resolution: WindowResolution::new(1280.0, 960.0),
                    ..default()
                });
            },
        );
        assert_eq!(mismatches, [], "{cli:?}");
    }
}

/// Under grow-left the compositor copies the composited core (9.3.10).
/// Every object of the busy town is in the core, so everything outside it
/// is the canvas grow-left alone draws (the ring's own objects are M2-07's
/// tests below).
#[test]
fn under_grow_left_the_core_carries_the_composite() {
    let grow = CliSetting::Enable("display.grow_left".into());
    let mut both = shell_with(busy_town(), &launch(&[grow.clone(), on()]), 1920.0, 1080.0);
    let mut alone = shell_with(busy_town(), &launch(&[grow]), 1920.0, 1080.0);
    let timeline = walk(40);
    let mut differed = 0;
    for index in 0..timeline.len() {
        both.step(timeline.frame(index));
        alone.step(timeline.frame(index));
        let (faithful, shown, size, _) = images(&mut both);
        let (_, plain, plain_size, _) = images(&mut alone);
        assert_eq!(size, plain_size, "frame {index}");
        if size == FRAME {
            continue;
        }
        let canvas =
            Canvas::for_window(1920, 1080, u5_runtime::enhancements::PixelAspect::Dos).unwrap();
        assert_eq!(size, UVec2::new(canvas.w, canvas.h));
        let (cx, cy, side) = compose::core_rect(&canvas.for_column(ColumnView::default(), false));
        let (composited, _) = expected_composite(&both, &faithful, true);
        let w = canvas.w as usize;
        for y in 0..canvas.h as usize {
            for x in 0..w {
                let at = (y * w + x) * 4;
                let in_core = (cx..cx + side).contains(&x) && (cy..cy + side).contains(&y);
                if in_core {
                    let from =
                        ((VIEWPORT_ORIGIN + y - cy) * FRAME.x as usize + VIEWPORT_ORIGIN + x - cx)
                            * 4;
                    assert_eq!(
                        shown[at..at + 3],
                        composited[from..from + 3],
                        "frame {index}: core ({x}, {y})"
                    );
                } else {
                    assert_eq!(
                        shown[at..at + 4],
                        plain[at..at + 4],
                        "frame {index}: ({x}, {y}) outside the core"
                    );
                }
            }
        }
        differed += usize::from(shown != plain);
    }
    assert!(differed > 0, "the core was composited");
}

#[test]
fn the_menus_changes_are_followed() {
    let mut shell = shell_with(busy_town(), &launch(&[on()]), 1280.0, 960.0);
    shell.step(&[]);
    let masks = shell.app.world().resource::<CompositeMasks>().clone();
    let built = masks.get().map(|masks| masks as *const SpriteMasks);
    assert!(built.is_some());
    let change = |shell: &mut Shell, composite: bool, halo: bool| {
        {
            let mut display = shell.app.world_mut().resource_mut::<Display>();
            display.0.light.composite = composite;
            display.0.light.halo = halo;
        }
        shell.step(&[]);
        shell.step(&[]);
    };
    let check = |shell: &mut Shell, halo: bool| {
        let (faithful, shown, _, _) = images(shell);
        let (expected, changed) = expected_composite(shell, &faithful, halo);
        assert!(changed > 0);
        assert_eq!(shown, expected, "halo {halo}");
    };
    change(&mut shell, true, false);
    assert_eq!(orders(&shell), Some(vec![StepOrder::COMPOSITE]));
    check(&mut shell, false);
    change(&mut shell, false, false);
    assert_eq!(orders(&shell), Some(vec![]));
    let (faithful, shown, _, faithful_handle) = images(&mut shell);
    assert!(faithful_handle);
    assert_eq!(faithful, shown, "off: the faithful image again");
    change(&mut shell, true, true);
    assert_eq!(orders(&shell), Some(vec![StepOrder::COMPOSITE]));
    check(&mut shell, true);
    // The masks were built once and kept.
    let now = shell.app.world().resource::<CompositeMasks>().clone();
    assert_eq!(now.get().map(|masks| masks as *const SpriteMasks), built);
}

// --- The ring (P5b, 9.4.4; build plan M2-07). --------------------------------

/// The canvas at 1920x1080 (480x216) and its party tile.
fn canvas_1080() -> Canvas {
    Canvas::for_window(1920, 1080, u5_runtime::enhancements::PixelAspect::Dos).unwrap()
}

/// The 16x16 cell at offset `(dx, dy)` from the party on a canvas image.
fn canvas_cell(rgba: &[u8], canvas: &Canvas, dx: isize, dy: isize) -> Vec<[u8; 3]> {
    let x0 = canvas.party_x as isize + 16 * dx;
    let y0 = canvas.party_y as isize + 16 * dy;
    (0..TILE_ATLAS_TILE_PIXELS)
        .map(|i| {
            let (x, y) = (x0 as usize + i % 16, y0 as usize + i / 16);
            let at = (y * canvas.w as usize + x) * 4;
            [rgba[at], rgba[at + 1], rgba[at + 2]]
        })
        .collect()
}

/// A person two cells east of the party, in the core, and one eight cells
/// east, in the ring, both on the open floor.
fn twin_town() -> PlayState {
    town_with(&[object(PERSON, 5 + 2, 5), object(PERSON, 5 + 8, 5 - 2)])
}

fn grow() -> CliSetting {
    CliSetting::Enable("display.grow_left".into())
}

#[test]
fn a_ring_sprite_equals_the_same_sprite_on_the_same_terrain_in_the_core() {
    let canvas = canvas_1080();
    for halo in [true, false] {
        let mut cli = vec![grow(), on()];
        if !halo {
            cli.push(set("display.composite_halo", "false"));
        }
        let mut both = shell_with(twin_town(), &launch(&cli), 1920.0, 1080.0);
        let mut alone = shell_with(twin_town(), &launch(&[grow()]), 1920.0, 1080.0);
        for _ in 0..2 {
            both.step(&[]);
            alone.step(&[]);
        }
        let objects = &both
            .app
            .world()
            .resource::<VisualState>()
            .state
            .active_objects;
        assert_eq!((objects[1].x, objects[1].y), (7, 5));
        assert_eq!((objects[2].x, objects[2].y), (13, 3));
        let (_, shown, size, _) = images(&mut both);
        let (_, plain, _, _) = images(&mut alone);
        assert_eq!(size, UVec2::new(canvas.w, canvas.h));
        let core = canvas_cell(&shown, &canvas, 2, 0);
        let ring = canvas_cell(&shown, &canvas, 8, -2);
        assert_eq!(ring, core, "halo {halo}: the ring sprite is the core's");
        assert_ne!(
            ring,
            canvas_cell(&plain, &canvas, 8, -2),
            "the ring composites"
        );
        assert_ne!(
            core,
            canvas_cell(&plain, &canvas, 2, 0),
            "the core composites"
        );
    }
}

/// With the key off the ring draws its objects boxed, exactly as M1: the
/// grow-left step has no composite and the ring cell is the atlas tile.
#[test]
fn with_the_key_off_the_ring_objects_keep_their_box() {
    let launch = launch(&[grow()]);
    let step = GrowLeftCompose::of(&crate::display::launch_display(&launch));
    assert_eq!(step.composite_halo(), None);
    let mut shell = shell_with(twin_town(), &launch, 1920.0, 1080.0);
    shell.step(&[]);
    assert!(!shell.app.world().contains_resource::<CompositeMasks>());
    let (_, shown, _, _) = images(&mut shell);
    let visual = shell.app.world().resource::<VisualState>();
    let palette = crate::visual_palette(visual.atlas.depth);
    let tile = visual
        .atlas
        .tile_pixels(ACTOR_TILE_BANK_BASE + usize::from(PERSON))
        .unwrap();
    let boxed: Vec<[u8; 3]> = tile.iter().map(|&i| palette[usize::from(i)]).collect();
    assert_eq!(canvas_cell(&shown, &canvas_1080(), 8, -2), boxed);
}

/// The ring flash (compose.rs `flash_ring`) inverts the composited ring:
/// with the flash on, every ring pixel is the palette XOR of the same pixel
/// with it off, composited sprites included.
#[test]
fn the_ring_flash_applies_after_the_composite() {
    let atlas = boxed_atlas(TileGraphicsDepth::Ega16);
    let masks = SpriteMasks::from_atlas(&atlas);
    let mut state = twin_town();
    state
        .render_top_down_base_frame(VIEWPORT_PLAYER_ROW, &atlas)
        .unwrap();
    let canvas = canvas_1080();
    let frame = vec![0u8; (FRAME.x * FRAME.y * 4) as usize];
    let compose = |state: &PlayState, composite: Option<(&SpriteMasks, bool)>| {
        crate::display::grow::compose_for_state(
            &frame,
            state,
            &atlas,
            &canvas,
            crate::display::grow::ComposeOptions {
                light: u5_runtime::PeekLightPolicy::Scaled,
                bands: None,
                crawl: None,
                column: ColumnView::default(),
                centre_party: false,
                composite,
            },
        )
        .unwrap()
    };
    let plain = compose(&state, None);
    let still = compose(&state, Some((&masks, true)));
    assert_ne!(
        canvas_cell(&still, &canvas, 8, -2),
        canvas_cell(&plain, &canvas, 8, -2),
        "the ring sprite composites"
    );
    let mut flashing = state.clone();
    flashing.presentation_camp_flash_inverted = true;
    assert!(crate::display::grow::flash_active(&flashing));
    let flashed = compose(&flashing, Some((&masks, true)));
    let playback = u5_runtime::potion_flash_playback(0).expect("the flash playback");
    let (mx0, my0, mx1, my1) = canvas.map_window();
    let (cx, cy, side) = compose::core_rect(&canvas);
    let w = canvas.w as usize;
    for y in my0 as usize..=my1 as usize {
        for x in mx0 as usize..=mx1 as usize {
            if (cx..cx + side).contains(&x) && (cy..cy + side).contains(&y) {
                continue;
            }
            let at = (y * w + x) * 4;
            let index = crate::ega_palette_index_from_rgba(&still[at..at + 4]);
            let inverted = crate::EGA_PALETTE_RGB[usize::from(index ^ playback.palette_xor_mask)];
            assert_eq!(flashed[at..at + 3], inverted, "({x}, {y})");
        }
    }
}

/// The menu switching compositing on and off in play re-registers the
/// grow-left step with or without the ring composite.
#[test]
fn the_menu_turns_the_ring_composite_on_and_off() {
    let canvas = canvas_1080();
    let mut shell = shell_with(twin_town(), &launch(&[grow(), on()]), 1920.0, 1080.0);
    let mut alone = shell_with(twin_town(), &launch(&[grow()]), 1920.0, 1080.0);
    alone.step(&[]);
    let (_, plain, _, _) = images(&mut alone);
    let boxed = canvas_cell(&plain, &canvas, 8, -2);
    let ring = |shell: &mut Shell| {
        shell.step(&[]);
        shell.step(&[]);
        let (_, shown, _, _) = images(shell);
        canvas_cell(&shown, &canvas, 8, -2)
    };
    let composited = ring(&mut shell);
    assert_ne!(composited, boxed);
    let set_composite = |shell: &mut Shell, on: bool| {
        let mut display = shell.app.world_mut().resource_mut::<Display>();
        display.0.light.composite = on;
    };
    set_composite(&mut shell, false);
    assert_eq!(ring(&mut shell), boxed, "off: boxed as M1");
    set_composite(&mut shell, true);
    assert_eq!(ring(&mut shell), composited, "on again");
    // A grow-left session started without compositing gains it from the
    // menu too (the step builds its own masks if the shared ones are not
    // there yet).
    set_composite(&mut alone, true);
    assert_eq!(ring(&mut alone), composited, "switched on in play");
}

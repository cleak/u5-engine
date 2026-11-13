//! M3-13 dungeon tests (10.2.3, 10.2.6, 10.2.7, 10.2.9 L6 and L7): the
//! stack at the three canvases, whole-pixel glyph scaling, the glyph
//! policy, remembered cells only, panels only with evidence, `Underworld`
//! only after a visit, the wrap rows and the stack toggles. Every colour is
//! taken from the engine's own `draw_dungeon_view_glyph` and palette, so no
//! colour is pinned here (D-M3.1).

use u5_runtime::enhancements::{GemMapToggles, GlyphPolicy, trajectory_digest};
use u5_runtime::test_fixtures::{dungeon_state, open_dungeon_record, synthetic_tile_atlas};
use u5_runtime::{
    CH_FONT_LEN, Direction, FixedCellFont, IBM_CH_FILE, PlayState, RUNES_CH_FILE,
    TileGraphicsDepth, ViewOverlay, ViewOverlayKind, ViewOverlayMode, dungeon_cell_index,
    parse_ch_font,
};

use super::*;
use crate::MAP_PALETTE;
use crate::gem::knowledge::{DungeonMark, KnowEvent, MarkKind};
use crate::gem::layout::{DUNGEON_PANEL_TABLE, DungeonLayout, GemCanvas, GlyphScale};
use crate::gem::raster::{
    DUNGEON_EDGE_GREY, DungeonLuts, GemImage, GemRender, MapArt, PARTY_YELLOW,
    dungeon_glyph_pixels, dungeon_panel_levels, facing_arrow, render_gem_map,
};
use crate::gem::shown::{GemShown, GemViewState, TownInputs};

const C43: GemCanvas = GemCanvas { w: 320, h: 200 };
const C169: GemCanvas = GemCanvas { w: 512, h: 240 };
const C219: GemCanvas = GemCanvas { w: 688, h: 240 };
const WALL: u8 = 0xB0;
const LADDER_DOWN: u8 = 0x20;
const LADDER_UP: u8 = 0x10;
const BLACK: [u8; 4] = [0, 0, 0, 255];

fn font(seed: u8) -> FixedCellFont {
    let bytes = (0..CH_FONT_LEN)
        .map(|i| {
            let (code, row) = ((i / 8) as u8, (i % 8) as u8);
            (code.wrapping_mul(31) ^ row.wrapping_mul(0x11) ^ seed) | 0x81
        })
        .collect::<Vec<_>>();
    let name = if seed == 0 {
        IBM_CH_FILE
    } else {
        RUNES_CH_FILE
    };
    parse_ch_font(&bytes, name).unwrap()
}

fn art() -> MapArt {
    MapArt::build(&synthetic_tile_atlas(TileGraphicsDepth::Ega16)).with_fonts(font(0), font(0x5A))
}

fn bare_art() -> MapArt {
    MapArt::build(&synthetic_tile_atlas(TileGraphicsDepth::Ega16))
}

fn on() -> GemMapToggles {
    GemMapToggles {
        enhanced: true,
        ..GemMapToggles::default()
    }
}

fn rgba(rgb: [u8; 3]) -> [u8; 4] {
    [rgb[0], rgb[1], rgb[2], 255]
}

fn set(state: &mut PlayState, level: u8, x: usize, y: usize, byte: u8) {
    state.grid[dungeon_cell_index(level, x, y)] = byte;
}

/// Every level all wall except, on `level`, a 2 x 2 room at (1..=2,
/// 1..=2); the party on (1, 1) facing north, a dungeon gem view up. The
/// flood is the room and its ring of walls, x and y in 0..=3.
fn cave(level: u8) -> PlayState {
    let mut state = dungeon_state(open_dungeon_record(), level, 1, 1);
    for lvl in 0..8 {
        for y in 0..8 {
            for x in 0..8 {
                set(&mut state, lvl, x, y, WALL);
            }
        }
    }
    for (x, y) in [(1, 1), (2, 1), (1, 2), (2, 2)] {
        set(&mut state, level, x, y, 0x00);
    }
    state.player.facing = Direction::North;
    state.active_view_overlay = Some(ViewOverlay {
        title: String::new(),
        text_map: String::new(),
        kind: ViewOverlayKind::Dungeon { level },
        mode: ViewOverlayMode::GemView,
    });
    state
}

fn record(state: &PlayState) -> u8 {
    match state.area {
        u5_runtime::Area::Dungeon { scene, .. } => scene.record as u8,
        _ => unreachable!("a dungeon"),
    }
}

fn remember(know: &mut GemKnowledge, record: u8, level: u8, cells: &[(u8, u8)]) {
    let mut rows = [0u8; 8];
    for &(x, y) in cells {
        rows[usize::from(y)] |= 1 << x;
    }
    know.apply(&KnowEvent::DungeonCellsSeen {
        record,
        level,
        rows,
    });
}

fn open(
    state: &PlayState,
    canvas: GemCanvas,
    know: &GemKnowledge,
    glyphs: GlyphPolicy,
) -> GemViewState {
    GemViewState::open_all(
        state,
        canvas,
        100,
        TownInputs::default(),
        DungeonInputs {
            know: Some(know),
            glyphs,
        },
    )
}

fn shown(view: &GemViewState) -> &DungeonShown {
    match &view.shown {
        GemShown::Dungeon(shown) => shown,
        other => panic!("a dungeon view, got {other:?}"),
    }
}

fn render(
    state: &PlayState,
    know: &GemKnowledge,
    art: &MapArt,
    t: &GemMapToggles,
    view: &GemViewState,
) -> GemImage {
    let mut out = GemImage::default();
    assert_eq!(
        render_gem_map(state, know, art, t, view, &mut out),
        GemRender::Dungeon
    );
    out
}

/// How a cell is lit and where: a stand-in for the brightness steps.
#[derive(Clone, Copy)]
struct Light {
    lit: DungeonLit,
    core: bool,
}

/// The expected pixel `(i, j)` of a cell drawing `drawn` at `cell` px, at
/// 70% and `light`: the painter's own glyph, scaled.
fn expected(art: &MapArt, drawn: u8, cell: u32, i: u32, j: u32, light: &Light) -> [u8; 4] {
    let scale = GlyphScale::for_cell(cell);
    let glyph = dungeon_glyph_pixels(drawn, art.fonts());
    let at = scale.source(j) as usize * 8 + scale.source(i) as usize;
    let rgb = MAP_PALETTE[usize::from(glyph[at])];
    rgba(DungeonLuts::new(70).colour(rgb, light.lit, light.core))
}

/// The canvas position of cell `(x, y)` of the panel showing `level`.
fn cell_origin(
    layout: &DungeonLayout,
    shown: &DungeonShown,
    level: i32,
    x: u8,
    y: u8,
) -> (i32, i32) {
    let (index, _) = dungeon_panel_levels(shown, layout, 0)
        .into_iter()
        .find(|&(_, l)| l == level)
        .expect("a panel shows the level");
    let (px, py) = layout.panel_origin(index);
    let c = layout.cell as i32;
    (px + i32::from(x) * c, py + i32::from(y) * c)
}

/// Every pixel of cell `(x, y)` of `level`'s panel.
fn cell_pixels(
    out: &GemImage,
    layout: &DungeonLayout,
    shown: &DungeonShown,
    level: i32,
    (x, y): (u8, u8),
) -> Vec<(u32, u32, [u8; 4])> {
    let (x0, y0) = cell_origin(layout, shown, level, x, y);
    let mut pixels = Vec::new();
    for j in 0..layout.cell {
        for i in 0..layout.cell {
            let p = out
                .pixel((x0 + i as i32) as u32, (y0 + j as i32) as u32)
                .unwrap();
            pixels.push((i, j, p));
        }
    }
    pixels
}

/// Whether cell `(x, y)` of `level`'s panel is `drawn`'s glyph via `lut`.
#[allow(clippy::too_many_arguments)]
fn cell_is(
    out: &GemImage,
    art: &MapArt,
    layout: &DungeonLayout,
    shown: &DungeonShown,
    level: i32,
    cell: (u8, u8),
    drawn: u8,
    light: &Light,
) -> bool {
    cell_pixels(out, layout, shown, level, cell)
        .into_iter()
        .all(|(i, j, p)| p == expected(art, drawn, layout.cell, i, j, light))
}

fn cell_black(
    out: &GemImage,
    layout: &DungeonLayout,
    shown: &DungeonShown,
    level: i32,
    cell: (u8, u8),
) -> bool {
    cell_pixels(out, layout, shown, level, cell)
        .into_iter()
        .all(|(.., p)| p == BLACK)
}

fn map_lut() -> Light {
    Light {
        lit: DungeonLit::Flood,
        core: true,
    }
}

fn remembered_lut() -> Light {
    Light {
        lit: DungeonLit::Remembered,
        core: true,
    }
}

#[test]
fn the_shown_set_is_the_flood_plus_remembered_cells() {
    let state = cave(2);
    let rec = record(&state);
    let mut know = GemKnowledge::new();
    remember(&mut know, rec, 2, &[(6, 6)]);
    remember(&mut know, rec, 1, &[(4, 4), (5, 4)]);
    // Another dungeon's bits are never read.
    remember(&mut know, (rec + 1) % 8, 3, &[(0, 0)]);
    let view = open(&state, C169, &know, GlyphPolicy::Perceived);
    let s = shown(&view);
    let flood: Vec<_> = s
        .cells(2)
        .filter(|c| c.2.lit == DungeonLit::Flood)
        .map(|c| (c.0, c.1))
        .collect();
    assert_eq!(flood.len(), 16);
    assert!(flood.iter().all(|&(x, y)| x <= 3 && y <= 3));
    assert_eq!(s.cell(2, 6, 6).map(|c| c.lit), Some(DungeonLit::Remembered));
    assert_eq!(s.cells(2).count(), 17);
    assert_eq!(s.cells(1).count(), 2);
    assert_eq!(s.cells(3).count(), 0, "another dungeon's memory");
    for lvl in [0, 4, 5, 6, 7] {
        assert_eq!(s.cells(lvl).count(), 0);
    }
    // No record: the flood alone.
    let bare = GemViewState::open(&state, C169, 100);
    assert_eq!(shown(&bare).cells(2).count(), 16);
    assert_eq!(shown(&bare).cells(1).count(), 0);
}

#[test]
fn the_stack_at_the_three_canvases() {
    let state = cave(2);
    let rec = record(&state);
    let mut know = GemKnowledge::new();
    for lvl in 0..8 {
        remember(&mut know, rec, lvl, &[(5, 5)]);
    }
    let art = art();
    for (canvas, panels, cell, levels) in [
        (C43, 1u8, 24u32, vec![2]),
        (C169, 3, 20, vec![1, 2, 3]),
        (C219, 5, 16, vec![0, 1, 2, 3, 4]),
    ] {
        let view = open(&state, canvas, &know, GlyphPolicy::Perceived);
        let s = shown(&view);
        let layout = DungeonLayout::new(canvas, true).unwrap();
        assert_eq!((layout.panels, layout.cell), (panels, cell));
        let got: Vec<_> = dungeon_panel_levels(s, &layout, 0)
            .into_iter()
            .map(|p| p.1)
            .collect();
        assert_eq!(got, levels, "{canvas:?}");
        let out = render(&state, &know, &art, &on(), &view);
        assert_eq!((out.w, out.h), (canvas.w, canvas.h));
        assert!(out.rgba.chunks(4).all(|p| p[3] == 255), "opaque");
        for &level in &levels {
            // Remembered cells, on the current level and the others: 70%
            // before the map's 70%.
            assert!(
                cell_is(
                    &out,
                    &art,
                    &layout,
                    s,
                    level,
                    (5, 5),
                    WALL,
                    &remembered_lut()
                ),
                "{canvas:?} level {level}"
            );
        }
        // A flood wall cell of the current level at the map brightness.
        assert!(cell_is(&out, &art, &layout, s, 2, (3, 3), WALL, &map_lut()));
    }
}

#[test]
fn glyphs_scale_by_whole_pixels_at_16_20_and_24() {
    assert_eq!(GlyphScale::for_cell(16), GlyphScale { factor: 2, pad: 0 });
    assert_eq!(GlyphScale::for_cell(20), GlyphScale { factor: 2, pad: 2 });
    assert_eq!(GlyphScale::for_cell(24), GlyphScale { factor: 3, pad: 0 });
    // 20 px: the first and last 4 px repeat the edge glyph pixels, every
    // other glyph pixel is a 2 x 2 block.
    let s = GlyphScale::for_cell(20);
    let sources: Vec<_> = (0..20).map(|i| s.source(i)).collect();
    assert_eq!(
        sources,
        [0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 7, 7]
    );
    for cell in [16, 24] {
        let s = GlyphScale::for_cell(cell);
        for i in 0..cell {
            assert_eq!(s.source(i), i / (cell / 8));
        }
    }
    // Rendered: a ladder at each size is the painter's own glyph so scaled,
    // and it is not a flat block.
    let mut state = cave(2);
    set(&mut state, 2, 2, 2, LADDER_DOWN);
    let know = GemKnowledge::new();
    let art = art();
    for canvas in [C43, C169, C219] {
        let view = open(&state, canvas, &know, GlyphPolicy::Perceived);
        let s = shown(&view);
        let layout = DungeonLayout::new(canvas, true).unwrap();
        let out = render(&state, &know, &art, &on(), &view);
        assert!(cell_is(
            &out,
            &art,
            &layout,
            s,
            2,
            (2, 2),
            LADDER_DOWN,
            &map_lut()
        ));
        let mut colours: Vec<_> = cell_pixels(&out, &layout, s, 2, (2, 2))
            .into_iter()
            .map(|p| p.2)
            .collect();
        colours.sort_unstable();
        colours.dedup();
        assert!(colours.len() > 1, "{canvas:?}");
    }
}

#[test]
fn perceived_draws_secret_doors_as_wall_and_hidden_traps_as_passage() {
    let mut state = cave(2);
    // A secret door in the ring, the hidden pits and a bomb trap in the room.
    set(&mut state, 2, 0, 1, 0xD3);
    set(&mut state, 2, 2, 1, 0x61);
    set(&mut state, 2, 1, 2, 0x69);
    set(&mut state, 2, 2, 2, 0x62);
    let rec = record(&state);
    let mut know = GemKnowledge::new();
    // A remembered secret door and hidden pit on the level above.
    set(&mut state, 1, 4, 4, 0xD5);
    set(&mut state, 1, 5, 4, 0x61);
    remember(&mut know, rec, 1, &[(4, 4), (5, 4)]);
    let art = art();
    let layout = DungeonLayout::new(C169, true).unwrap();
    let view = open(&state, C169, &know, GlyphPolicy::Perceived);
    let s = shown(&view);
    let out = render(&state, &know, &art, &on(), &view);
    assert_eq!(s.cell(2, 0, 1).unwrap().drawn, 0xB3);
    assert!(cell_is(&out, &art, &layout, s, 2, (0, 1), 0xB3, &map_lut()));
    for (cell, passage) in [((2, 1), 0x00), ((1, 2), 0x08), ((2, 2), 0x00)] {
        assert_eq!(s.cell(2, cell.0, cell.1).unwrap().drawn, passage);
        assert!(cell_is(
            &out,
            &art,
            &layout,
            s,
            2,
            cell,
            passage,
            &map_lut()
        ));
    }
    assert!(
        cell_black(&out, &layout, s, 2, (2, 1)),
        "passage draws nothing"
    );
    assert_eq!(s.cell(1, 4, 4).unwrap().drawn, 0xB5);
    assert_eq!(s.cell(1, 5, 4).unwrap().drawn, 0x00);
    assert!(cell_is(
        &out,
        &art,
        &layout,
        s,
        1,
        (4, 4),
        0xB5,
        &remembered_lut()
    ));
    assert!(cell_black(&out, &layout, s, 1, (5, 4)));
    // Under gem-faithful they show (a labelled deviation).
    let faithful = open(&state, C169, &know, GlyphPolicy::GemFaithful);
    let f = shown(&faithful);
    let fout = render(&state, &know, &art, &on(), &faithful);
    for (cell, raw) in [
        ((0, 1), 0xD3),
        ((2, 1), 0x61),
        ((1, 2), 0x69),
        ((2, 2), 0x62),
    ] {
        assert_eq!(f.cell(2, cell.0, cell.1).unwrap().drawn, raw);
        assert!(cell_is(&fout, &art, &layout, f, 2, cell, raw, &map_lut()));
        let perceived = s.cell(2, cell.0, cell.1).unwrap().drawn;
        assert_ne!(
            dungeon_glyph_pixels(raw, art.fonts()),
            dungeon_glyph_pixels(perceived, art.fonts()),
            "{raw:#04x} looks different from its perceived form"
        );
    }
    // Found: the remembered secret door draws as a door and the pit as a
    // pit, though the level's bytes are the hidden forms.
    for (x, kind) in [(4, MarkKind::SecretDoor), (5, MarkKind::Trap)] {
        know.apply(&KnowEvent::DungeonMarked(DungeonMark {
            record: rec,
            level: 1,
            x,
            y: 4,
            kind,
        }));
    }
    let found = open(&state, C169, &know, GlyphPolicy::Perceived);
    let f = shown(&found);
    assert_eq!(f.cell(1, 4, 4).unwrap().drawn, 0xE5);
    assert_eq!(f.cell(1, 5, 4).unwrap().drawn, 0x60);
    let fout = render(&state, &know, &art, &on(), &found);
    assert!(cell_is(
        &fout,
        &art,
        &layout,
        f,
        1,
        (4, 4),
        0xE5,
        &remembered_lut()
    ));
    assert!(cell_is(
        &fout,
        &art,
        &layout,
        f,
        1,
        (5, 4),
        0x60,
        &remembered_lut()
    ));
}

#[test]
fn no_cell_of_another_level_is_drawn_unless_remembered() {
    let mut state = cave(2);
    for lvl in [1, 3] {
        for y in 0..8 {
            for x in 0..8 {
                let byte = if (x + y) % 2 == 0 { 0x30 } else { 0x40 };
                set(&mut state, lvl, x, y, byte);
            }
        }
    }
    let rec = record(&state);
    let mut know = GemKnowledge::new();
    remember(&mut know, rec, 1, &[(6, 6)]);
    remember(&mut know, rec, 3, &[(0, 7)]);
    let art = art();
    let layout = DungeonLayout::new(C169, true).unwrap();
    let view = open(&state, C169, &know, GlyphPolicy::Perceived);
    let s = shown(&view);
    let out = render(&state, &know, &art, &on(), &view);
    for (level, keep) in [(1, (6u8, 6u8)), (3, (0, 7))] {
        for y in 0..8 {
            for x in 0..8 {
                let black = cell_black(&out, &layout, s, level, (x, y));
                assert_eq!(black, (x, y) != keep, "level {level} cell {x},{y}");
            }
        }
    }
    // The current level's unremembered cells outside the flood are black.
    for y in 4..8 {
        for x in 4..8 {
            assert!(cell_black(&out, &layout, s, 2, (x, y)));
        }
    }
}

#[test]
fn a_panel_never_appears_without_evidence() {
    let state = cave(2);
    let know = GemKnowledge::new();
    let art = art();
    let layout = DungeonLayout::new(C169, true).unwrap();
    let view = open(&state, C169, &know, GlyphPolicy::Perceived);
    let s = shown(&view);
    assert_eq!(s.slot(1), DungeonSlot::Empty);
    assert_eq!(s.slot(3), DungeonSlot::Empty);
    assert_eq!((s.title(1), s.title(3)), (None, None));
    let out = render(&state, &know, &art, &on(), &view);
    // The side slots are plain black: no edge, no title, no wrap.
    let centre = layout.panel_origin(1).0;
    for y in 0..240u32 {
        for x in 0..512u32 {
            if (centre - 1..=centre + 160).contains(&(x as i32)) {
                continue;
            }
            assert_eq!(out.pixel(x, y).unwrap(), BLACK, "{x},{y}");
        }
    }
    // A ladder down drawn in the flood is evidence for the level below: a
    // black panel with its title and edge; the level above stays empty.
    let mut ladder = state.clone();
    set(&mut ladder, 2, 2, 2, LADDER_DOWN);
    let view = open(&ladder, C169, &know, GlyphPolicy::Perceived);
    let s = shown(&view);
    assert_eq!(
        s.slot(3),
        DungeonSlot::Level {
            level: 3,
            cells: false
        }
    );
    assert_eq!(s.title(3).as_deref(), Some("Level 4"));
    assert_eq!(s.slot(1), DungeonSlot::Empty);
    let out = render(&ladder, &know, &art, &on(), &view);
    let (px, py) = layout.panel_origin(2);
    assert_eq!(
        out.pixel((px - 1) as u32, (py + 10) as u32).unwrap(),
        rgba(DUNGEON_EDGE_GREY)
    );
    for y in 0..8 {
        for x in 0..8 {
            assert!(
                cell_black(&out, &layout, s, 3, (x, y)),
                "the ladder shows no cell"
            );
        }
    }
    // A hidden pit (perceived as passage) is no evidence.
    let mut pit = state.clone();
    set(&mut pit, 2, 2, 2, 0x61);
    let view = open(&pit, C169, &know, GlyphPolicy::Perceived);
    assert_eq!(shown(&view).slot(3), DungeonSlot::Empty);
    // A visible pit is.
    set(&mut pit, 2, 2, 2, 0x60);
    let view = open(&pit, C169, &know, GlyphPolicy::Perceived);
    assert_eq!(
        shown(&view).slot(3),
        DungeonSlot::Level {
            level: 3,
            cells: false
        }
    );
    // Remembered cells are; a ladder up on that level leads one level
    // further up, but an evidence-only level leads nowhere.
    let mut know = GemKnowledge::new();
    remember(&mut know, record(&state), 1, &[(3, 3)]);
    let mut up = state.clone();
    set(&mut up, 1, 3, 3, LADDER_UP);
    set(&mut up, 0, 3, 3, LADDER_UP);
    let view = open(&up, C219, &know, GlyphPolicy::Perceived);
    let s = shown(&view);
    assert_eq!(
        s.slot(1),
        DungeonSlot::Level {
            level: 1,
            cells: true
        }
    );
    assert_eq!(
        s.slot(0),
        DungeonSlot::Level {
            level: 0,
            cells: false
        }
    );
    assert_eq!(s.slot(-1), DungeonSlot::Surface);
    assert_eq!(s.slot(4), DungeonSlot::Empty);
}

#[test]
fn titles_name_the_current_level_others_and_the_surface() {
    let state = cave(0);
    let mut know = GemKnowledge::new();
    let view = open(&state, C169, &know, GlyphPolicy::Perceived);
    let s = shown(&view);
    assert_eq!(s.title(-1).as_deref(), Some("Surface"));
    assert_eq!(s.title(0), Some(format!("{} level 1", s.name)));
    assert_eq!(s.title(1), None);
    remember(&mut know, record(&state), 1, &[(0, 0)]);
    let view = open(&state, C169, &know, GlyphPolicy::Perceived);
    assert_eq!(shown(&view).title(1).as_deref(), Some("Level 2"));
    // Drawn above all three panels, in EGA 15.
    let out = render(&state, &know, &art(), &on(), &view);
    let layout = DungeonLayout::new(C169, true).unwrap();
    for index in 0..3u8 {
        let (px, py) = layout.panel_origin(index);
        let any = (py - 13..py - 3).any(|y| {
            (px..px + 160).any(|x| out.pixel(x as u32, y as u32) == Some(rgba(MAP_PALETTE[15])))
        });
        assert!(any, "title over panel {index}");
    }
    // 4:3: no room above; the name goes in the left margin.
    let view = open(&state, C43, &know, GlyphPolicy::Perceived);
    let out = render(&state, &know, &art(), &on(), &view);
    let white = (6..26).any(|y| (0..64).any(|x| out.pixel(x, y) == Some(rgba(MAP_PALETTE[15]))));
    assert!(white);
}

#[test]
fn the_underworld_is_named_only_after_a_visit() {
    let mut state = cave(7);
    let know = GemKnowledge::new();
    let art = art();
    let layout = DungeonLayout::new(C169, true).unwrap();
    // No evidence: nothing below the deepest level.
    let view = open(&state, C169, &know, GlyphPolicy::Perceived);
    assert_eq!(shown(&view).slot(8), DungeonSlot::Empty);
    // A ladder down leads there: a slot, but no title before the visit.
    set(&mut state, 7, 2, 2, LADDER_DOWN);
    let view = open(&state, C169, &know, GlyphPolicy::Perceived);
    let s = shown(&view);
    assert_eq!(s.slot(8), DungeonSlot::Underworld { named: false });
    assert_eq!(s.title(8), None);
    let out = render(&state, &know, &art, &on(), &view);
    let (px, _) = layout.panel_origin(2);
    let column_black = |out: &GemImage| {
        (0..240).all(|y| (px..px + 160).all(|x| out.pixel(x as u32, y as u32) == Some(BLACK)))
    };
    assert!(column_black(&out), "no Underworld title before the visit");
    // After the visit.
    let mut visited = GemKnowledge::new();
    visited.apply(&KnowEvent::UnderworldVisited);
    let view = open(&state, C169, &visited, GlyphPolicy::Perceived);
    let s = shown(&view);
    assert_eq!(s.slot(8), DungeonSlot::Underworld { named: true });
    assert_eq!(s.title(8).as_deref(), Some("Underworld"));
    let out = render(&state, &visited, &art, &on(), &view);
    assert!(!column_black(&out));
    // Visited, but nothing drawn leads there: no slot and no title.
    let view = open(&cave(7), C169, &visited, GlyphPolicy::Perceived);
    assert_eq!(shown(&view).slot(8), DungeonSlot::Empty);
    assert_eq!(shown(&view).title(8), None);
}

#[test]
fn wrap_rows_repeat_the_level_dimmed_and_go_with_the_toggle() {
    let state = cave(2);
    let know = GemKnowledge::new();
    let art = bare_art(); // no titles, no arrowhead
    let layout = DungeonLayout::new(C169, true).unwrap();
    let view = open(&state, C169, &know, GlyphPolicy::Perceived);
    let (px, py) = layout.panel_origin(1);
    let p = layout.panel_px() as i32;
    let c = layout.cell as i32;
    let ghost = Light {
        lit: DungeonLit::Flood,
        core: false,
    };
    // On: level cell (3, 1) wraps to 8 cells down, at 30%.
    let out = render(&state, &know, &art, &on(), &view);
    let mut checked = 0;
    for j in 0..layout.cell {
        for i in 0..layout.cell {
            let (x, y) = (px + 3 * c + i as i32, py + 9 * c + j as i32);
            if y >= 240 {
                continue;
            }
            assert_eq!(
                out.pixel(x as u32, y as u32).unwrap(),
                expected(&art, WALL, layout.cell, i, j, &ghost)
            );
            checked += 1;
        }
    }
    assert!(checked > 0);
    assert_eq!(
        out.pixel((px - 1) as u32, (py + 5) as u32).unwrap(),
        rgba(DUNGEON_EDGE_GREY)
    );
    // Off: nothing outside the panels' own 8 x 8.
    let mut t = on();
    t.opts.wrap_rows = false;
    let out = render(&state, &know, &art, &t, &view);
    for y in 0..240 {
        for x in 0..512 {
            let inside = (py..py + p).contains(&y) && (px..px + p).contains(&x);
            if !inside {
                assert_eq!(out.pixel(x as u32, y as u32).unwrap(), BLACK, "{x},{y}");
            }
        }
    }
    // 4:3 wraps at the sides too: cell (0, 1) repeats 8 cells to its right.
    let view = open(&state, C43, &know, GlyphPolicy::Perceived);
    let layout = DungeonLayout::new(C43, true).unwrap();
    assert_eq!(layout.wrap_side(), 64);
    let out = render(&state, &know, &art, &on(), &view);
    let (px, py) = layout.panel_origin(0);
    let (x, y) = (px + 8 * 24 + 5, py + 24 + 5);
    assert_eq!(
        out.pixel(x as u32, y as u32).unwrap(),
        expected(&art, WALL, 24, 5, 5, &ghost)
    );
}

#[test]
fn with_the_stack_off_the_current_level_shows_alone_at_the_largest_size() {
    let state = cave(2);
    let rec = record(&state);
    let mut know = GemKnowledge::new();
    for lvl in 0..8 {
        remember(&mut know, rec, lvl, &[(5, 5)]);
    }
    let mut t = on();
    t.opts.dungeon_stack = false;
    let art = art();
    for (canvas, cell) in [(C43, 24), (C169, 28), (C219, 28)] {
        let layout = DungeonLayout::new(canvas, false).unwrap();
        assert_eq!((layout.panels, layout.cell), (1, cell), "{canvas:?}");
        let view = open(&state, canvas, &know, GlyphPolicy::Perceived);
        let s = shown(&view);
        assert_eq!(dungeon_panel_levels(s, &layout, 0), vec![(0, 2)]);
        let out = render(&state, &know, &art, &t, &view);
        assert!(cell_is(&out, &art, &layout, s, 2, (3, 3), WALL, &map_lut()));
    }
}

#[test]
fn the_party_is_a_yellow_arrowhead_pointing_its_facing() {
    let art = art();
    let text = font(0);
    let know = GemKnowledge::new();
    let layout = DungeonLayout::new(C169, true).unwrap();
    let mut pictures = Vec::new();
    for facing in [
        Direction::North,
        Direction::East,
        Direction::South,
        Direction::West,
    ] {
        let mut state = cave(2);
        state.player.facing = facing;
        let view = open(&state, C169, &know, GlyphPolicy::Perceived);
        let s = shown(&view);
        let out = render(&state, &know, &art, &on(), &view);
        let code = facing_arrow(facing);
        let mut picture = Vec::new();
        for (i, j, p) in cell_pixels(&out, &layout, s, 2, (1, 1)) {
            // The IBM.CH arrow at 2x, 2 px in, in EGA 14 on black.
            let (i, j) = (i as i32, j as i32);
            let ink = (2..18).contains(&i) && (2..18).contains(&j) && {
                let bits = text.glyph_row(code, ((j - 2) / 2) as usize).unwrap();
                bits & (0x80 >> ((i - 2) / 2)) != 0
            };
            let want = if ink { rgba(PARTY_YELLOW) } else { BLACK };
            assert_eq!(p, want, "{facing:?} {i},{j}");
            picture.push(p);
        }
        pictures.push(picture);
    }
    pictures.sort();
    pictures.dedup();
    assert_eq!(pictures.len(), 4, "each facing its own arrow");
}

#[test]
fn rendering_a_dungeon_is_pure_and_cleared_without_a_gem_view() {
    let state = cave(2);
    let know = GemKnowledge::new();
    let before = trajectory_digest(&state);
    let view = open(&state, C169, &know, GlyphPolicy::Perceived);
    let _ = render(&state, &know, &art(), &on(), &view);
    assert_eq!(trajectory_digest(&state), before);
    let mut out = GemImage::default();
    assert_eq!(
        render_gem_map(
            &state,
            &know,
            &art(),
            &GemMapToggles::default(),
            &view,
            &mut out
        ),
        GemRender::Cleared
    );
    let mut closed = state.clone();
    closed.active_view_overlay = None;
    assert_eq!(
        GemViewState::open(&closed, C169, 100).shown,
        GemShown::Empty
    );
    let mut peer = state.clone();
    peer.active_view_overlay.as_mut().unwrap().mode = ViewOverlayMode::PeerSpell;
    assert_eq!(GemViewState::open(&peer, C169, 100).shown, GemShown::Empty);
}

#[test]
fn the_panel_table_follows_the_general_rule() {
    for (canvas, panels, cell, gap) in DUNGEON_PANEL_TABLE {
        let table = DungeonLayout::new(canvas, true).unwrap();
        assert_eq!(table, DungeonLayout::general(canvas).unwrap(), "{canvas:?}");
        assert_eq!((table.panels, table.cell, table.gap), (panels, cell, gap));
        let n = u32::from(panels);
        let total = n * 8 * cell + (n - 1) * gap;
        if panels > 1 {
            assert_eq!(total, canvas.w, "edge to edge");
        }
        assert_eq!(table.origin.0 as u32, (canvas.w - total) / 2);
        assert_eq!(table.origin.1 as u32, (canvas.h - 8 * cell) / 2);
    }
    for (canvas, panels) in [
        (GemCanvas { w: 480, h: 216 }, 3),
        (GemCanvas { w: 688, h: 216 }, 5),
    ] {
        let l = DungeonLayout::new(canvas, true).unwrap();
        assert_eq!((l.panels, l.cell, l.gap), (panels, 16, 12));
    }
    assert_eq!(crate::gem::layout::dungeon_cell_size(15), None);
    assert_eq!(crate::gem::layout::dungeon_cell_size(25), Some(24));
    assert_eq!(crate::gem::layout::dungeon_cell_size(30), Some(28));
}

#[path = "links_tests.rs"]
mod links;

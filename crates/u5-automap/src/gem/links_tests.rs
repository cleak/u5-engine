//! M3-14 (10.2.6): the ladder link boxes need both ends shown, and PgUp
//! and PgDn page a lone panel through the levels that have one.

use super::*;
use crate::gem::raster::LINK_YELLOW;

/// Cell `(x, y)` of `level`'s panel on page `page`: its border is the link
/// box, and its inside is the glyph of `drawn` lit `light`.
#[allow(clippy::too_many_arguments)]
fn boxed_on_page(
    out: &GemImage,
    art: &MapArt,
    layout: &DungeonLayout,
    shown: &DungeonShown,
    page: i8,
    level: i32,
    (x, y): (u8, u8),
    drawn: u8,
    light: &Light,
) -> bool {
    let (index, _) = dungeon_panel_levels(shown, layout, page)
        .into_iter()
        .find(|&(_, l)| l == level)
        .expect("a panel shows the level");
    let (px, py) = layout.panel_origin(index);
    let c = layout.cell;
    let (x0, y0) = (px + i32::from(x) * c as i32, py + i32::from(y) * c as i32);
    (0..c).all(|j| {
        (0..c).all(|i| {
            let p = out
                .pixel((x0 + i as i32) as u32, (y0 + j as i32) as u32)
                .unwrap();
            let edge = i == 0 || j == 0 || i == c - 1 || j == c - 1;
            if edge {
                p == rgba(LINK_YELLOW)
            } else {
                p == expected(art, drawn, c, i, j, light)
            }
        })
    })
}

#[allow(clippy::too_many_arguments)]
fn boxed(
    out: &GemImage,
    art: &MapArt,
    layout: &DungeonLayout,
    shown: &DungeonShown,
    level: i32,
    cell: (u8, u8),
    drawn: u8,
    light: &Light,
) -> bool {
    boxed_on_page(out, art, layout, shown, 0, level, cell, drawn, light)
}

#[test]
fn a_link_box_needs_both_ends_shown() {
    // Level 2's flood has a ladder down at (2, 2); level 3 has the ladder
    // up under it.
    let mut state = cave(2);
    set(&mut state, 2, 2, 2, LADDER_DOWN);
    set(&mut state, 3, 2, 2, LADDER_UP);
    let rec = record(&state);
    let art = art();
    let layout = DungeonLayout::new(C169, true).unwrap();

    // The landing never seen: no box anywhere, the ladder is its glyph.
    let fresh = GemKnowledge::new();
    let view = open(&state, C169, &fresh, GlyphPolicy::Perceived);
    let s = shown(&view);
    assert!(!s.linked(2, 2, 2));
    assert!(!s.linked(3, 2, 2), "not shown at all");
    let out = render(&state, &fresh, &art, &on(), &view);
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
    for level in 0..8 {
        for (x, y, _) in s.cells(level) {
            assert!(!s.linked(level, x, y), "level {level} ({x}, {y})");
        }
    }

    // The landing remembered: both ends boxed, on their own panels.
    let mut know = GemKnowledge::new();
    remember(&mut know, rec, 3, &[(2, 2)]);
    let view = open(&state, C169, &know, GlyphPolicy::Perceived);
    let s = shown(&view);
    assert!(s.linked(2, 2, 2) && s.linked(3, 2, 2));
    let out = render(&state, &know, &art, &on(), &view);
    assert!(boxed(
        &out,
        &art,
        &layout,
        s,
        2,
        (2, 2),
        LADDER_DOWN,
        &map_lut()
    ));
    assert!(boxed(
        &out,
        &art,
        &layout,
        s,
        3,
        (2, 2),
        LADDER_UP,
        &remembered_lut()
    ));
    // No other cell is boxed: the flood's other cells are their glyphs.
    assert!(!s.linked(2, 1, 2) && !s.linked(2, 3, 3));
    assert!(cell_is(&out, &art, &layout, s, 2, (1, 2), 0x00, &map_lut()));

    // A remembered landing that is no way up does not join a ladder.
    let mut floor = state.clone();
    set(&mut floor, 3, 2, 2, 0x00);
    let view = open(&floor, C169, &know, GlyphPolicy::Perceived);
    assert!(!shown(&view).linked(2, 2, 2));
    assert!(!shown(&view).linked(3, 2, 2));
}

#[test]
fn on_4_3_a_remembered_end_off_screen_still_boxes() {
    let mut state = cave(2);
    set(&mut state, 2, 2, 2, LADDER_DOWN);
    set(&mut state, 3, 2, 2, LADDER_UP);
    set(&mut state, 2, 1, 2, LADDER_UP);
    set(&mut state, 1, 1, 2, LADDER_DOWN);
    let rec = record(&state);
    let mut know = GemKnowledge::new();
    remember(&mut know, rec, 3, &[(2, 2)]);
    let art = art();
    let layout = DungeonLayout::new(C43, true).unwrap();
    assert_eq!(layout.panels, 1);
    let view = open(&state, C43, &know, GlyphPolicy::Perceived);
    let s = shown(&view);
    let out = render(&state, &know, &art, &on(), &view);
    // Level 3 is off screen, but its landing is remembered.
    assert!(boxed(
        &out,
        &art,
        &layout,
        s,
        2,
        (2, 2),
        LADDER_DOWN,
        &map_lut()
    ));
    // The ladder up leads to level 2's unseen cell: no box.
    assert!(!s.linked(2, 1, 2));
    assert!(cell_is(
        &out,
        &art,
        &layout,
        s,
        2,
        (1, 2),
        LADDER_UP,
        &map_lut()
    ));
    // Paged down to level 4, the landing is boxed there too.
    let mut paged = view.clone();
    assert!(paged.turn_page(false, true));
    assert_eq!(paged.page, 1);
    let out = render(&state, &know, &art, &on(), &paged);
    assert!(boxed_on_page(
        &out,
        &art,
        &layout,
        s,
        1,
        3,
        (2, 2),
        LADDER_UP,
        &remembered_lut()
    ));
}

#[test]
fn a_visible_pit_links_to_its_landing_and_a_hidden_one_never() {
    let mut state = cave(2);
    set(&mut state, 2, 2, 2, 0x60);
    let rec = record(&state);
    let mut know = GemKnowledge::new();
    remember(&mut know, rec, 3, &[(2, 2)]);
    let art = art();
    let layout = DungeonLayout::new(C169, true).unwrap();
    let view = open(&state, C169, &know, GlyphPolicy::Perceived);
    let s = shown(&view);
    // The pit is boxed; its landing, plain floor, is not.
    assert!(s.linked(2, 2, 2));
    assert!(!s.linked(3, 2, 2));
    let out = render(&state, &know, &art, &on(), &view);
    assert!(boxed(&out, &art, &layout, s, 2, (2, 2), 0x60, &map_lut()));
    // A hidden pit is perceived as passage: no box, under either landing.
    for raw in [0x61, 0x69, 0x62] {
        let mut hidden = state.clone();
        set(&mut hidden, 2, 2, 2, raw);
        let view = open(&hidden, C169, &know, GlyphPolicy::Perceived);
        assert!(!shown(&view).linked(2, 2, 2), "{raw:#04x}");
        assert!(!shown(&view).linked(3, 2, 2), "{raw:#04x}");
        let drawn = shown(&view).cell(2, 2, 2).unwrap().drawn;
        assert_eq!(leads(drawn), (false, false), "{raw:#04x}: passage");
        let out = render(&hidden, &know, &art, &on(), &view);
        assert!(
            cell_is(
                &out,
                &art,
                &layout,
                shown(&view),
                2,
                (2, 2),
                drawn,
                &map_lut()
            ),
            "{raw:#04x}: its passage glyph, no box"
        );
    }
}

#[test]
fn links_stop_at_the_dungeon_s_ends() {
    // A ladder up on level 0 leads to the surface and a ladder down on
    // level 7 to the Underworld: neither has a shown other end.
    let mut top = cave(0);
    set(&mut top, 0, 2, 2, LADDER_UP);
    let know = GemKnowledge::new();
    let view = open(&top, C169, &know, GlyphPolicy::Perceived);
    assert!(!shown(&view).linked(0, 2, 2));
    let mut bottom = cave(7);
    set(&mut bottom, 7, 2, 2, LADDER_DOWN);
    let view = open(&bottom, C169, &know, GlyphPolicy::Perceived);
    assert!(!shown(&view).linked(7, 2, 2));
}

/// Level 2 with a ladder up and one down in its flood, level 5
/// remembered: levels 1 and 3 have (black) panels, level 4 none.
fn stack_with_a_gap() -> (PlayState, GemKnowledge) {
    let mut state = cave(2);
    set(&mut state, 2, 2, 1, LADDER_UP);
    set(&mut state, 2, 2, 2, LADDER_DOWN);
    let rec = record(&state);
    let mut know = GemKnowledge::new();
    remember(&mut know, rec, 5, &[(5, 5)]);
    (state, know)
}

#[test]
fn paging_skips_the_levels_without_a_panel() {
    let (state, know) = stack_with_a_gap();
    let view = open(&state, C43, &know, GlyphPolicy::Perceived);
    let s = shown(&view);
    assert!(s.has_panel(1) && s.has_panel(3) && s.has_panel(5));
    assert!(!s.has_panel(0) && !s.has_panel(4) && !s.has_panel(-1));
    assert_eq!(s.slot(-1), DungeonSlot::Surface, "a title, not a panel");
    // PgUp: level 1, then nothing further (level 0 empty, the surface no
    // panel).
    assert_eq!(s.page_step(0, true), -1);
    assert_eq!(s.page_step(-1, true), -1);
    // PgDn: level 3, then level 5 past the empty level 4, then nothing.
    assert_eq!(s.page_step(0, false), 1);
    assert_eq!(s.page_step(1, false), 3);
    assert_eq!(s.page_step(3, false), 3);
    // And back.
    assert_eq!(s.page_step(3, true), 1);
    assert_eq!(s.page_step(1, true), 0);

    let mut view = view.clone();
    let art = art();
    let layout = DungeonLayout::new(C43, true).unwrap();
    let current = format!("{} level 3", shown(&view).name);
    for (up, page, level, title) in [
        (false, 1, 3, "Level 4"),
        (false, 3, 5, "Level 6"),
        (true, 1, 3, "Level 4"),
        (true, 0, 2, current.as_str()),
        (true, -1, 1, "Level 2"),
    ] {
        assert!(view.turn_page(up, true));
        assert_eq!(view.page, page);
        let s = shown(&view);
        assert_eq!(
            dungeon_panel_levels(s, &layout, view.page),
            vec![(0, level)]
        );
        assert_eq!(s.title(level).as_deref(), Some(title));
        let out = render(&state, &know, &art, &on(), &view);
        if level == 5 {
            // The remembered level shows its cells on the lone panel.
            assert!(cell_is_on_page(
                &out,
                &art,
                &layout,
                s,
                page,
                5,
                (5, 5),
                WALL
            ));
        }
    }
    // At the top, PgUp does nothing.
    assert!(!view.turn_page(true, true));
    assert_eq!(view.page, -1);
}

/// Cell `(x, y)` of `level`'s panel on page `page` is `drawn` remembered.
#[allow(clippy::too_many_arguments)]
fn cell_is_on_page(
    out: &GemImage,
    art: &MapArt,
    layout: &DungeonLayout,
    shown: &DungeonShown,
    page: i8,
    level: i32,
    (x, y): (u8, u8),
    drawn: u8,
) -> bool {
    let (index, _) = dungeon_panel_levels(shown, layout, page)
        .into_iter()
        .find(|&(_, l)| l == level)
        .expect("a panel shows the level");
    let (px, py) = layout.panel_origin(index);
    let c = layout.cell;
    let (x0, y0) = (px + i32::from(x) * c as i32, py + i32::from(y) * c as i32);
    (0..c).all(|j| {
        (0..c).all(|i| {
            out.pixel((x0 + i as i32) as u32, (y0 + j as i32) as u32)
                == Some(expected(art, drawn, c, i, j, &remembered_lut()))
        })
    })
}

#[test]
fn only_a_one_panel_dungeon_map_pages() {
    let (state, know) = stack_with_a_gap();
    // 16:9 and 21:9 with the stack: several panels, no paging.
    for canvas in [C169, C219] {
        let mut view = open(&state, canvas, &know, GlyphPolicy::Perceived);
        assert!(!view.is_one_panel_dungeon(true));
        assert!(!view.turn_page(true, true));
        assert!(!view.turn_page(false, true));
        assert_eq!(view.page, 0);
        // With the stack off, the lone panel pages.
        assert!(view.is_one_panel_dungeon(false));
        assert!(view.turn_page(false, false));
        assert_eq!(view.page, 1);
    }
    // 4:3: one panel either way.
    let view = open(&state, C43, &know, GlyphPolicy::Perceived);
    assert!(view.is_one_panel_dungeon(true) && view.is_one_panel_dungeon(false));
    // Outdoors and in towns nothing pages.
    let world = u5_runtime::test_fixtures::world_state(
        u5_runtime::test_fixtures::open_world_grid(),
        100,
        100,
    );
    let mut world = world;
    world.active_view_overlay = Some(ViewOverlay {
        title: String::new(),
        text_map: String::new(),
        kind: ViewOverlayKind::Surface,
        mode: ViewOverlayMode::GemView,
    });
    let mut outdoor = GemViewState::open(&world, C43, 100);
    assert!(matches!(outdoor.shown, GemShown::Outdoor(_)));
    assert!(!outdoor.is_one_panel_dungeon(true));
    assert!(!outdoor.turn_page(true, true));
    assert_eq!(outdoor.page, 0);
}

#[test]
fn paging_is_pure() {
    let (state, know) = stack_with_a_gap();
    let digest = trajectory_digest(&state);
    let mut view = open(&state, C43, &know, GlyphPolicy::Perceived);
    let art = art();
    let first = render(&state, &know, &art, &on(), &view);
    assert!(view.turn_page(false, true));
    let paged = render(&state, &know, &art, &on(), &view);
    assert_ne!(first, paged, "another level");
    assert!(view.turn_page(true, true));
    assert_eq!(render(&state, &know, &art, &on(), &view), first);
    assert_eq!(trajectory_digest(&state), digest);
}

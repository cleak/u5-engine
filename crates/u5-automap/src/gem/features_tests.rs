//! M3-15 feature and mark tests (10.2.3 rule 2, 10.2.7, 10.2.8, 10.2.9 L1,
//! L3, L4, L8 and L9): every "?" sits on a shown cell whose drawn tile is
//! one of its kind's, with the neutral grey ring; nothing undiscovered has
//! a colour or a label; moongates only while the gate is up or once used;
//! hidden objects get nothing; one record bit changes one mark; the
//! gazetteer never creates a feature; the Codex is a place; with labels
//! off nothing is drawn.

use u5_runtime::enhancements::{GemMapToggles, GlyphPolicy};
use u5_runtime::test_fixtures::{
    dungeon_state, open_dungeon_record, open_world_grid, synthetic_tile_atlas, test_state,
    world_state,
};
use u5_runtime::{
    ActiveObject, Area, CH_FONT_LEN, Direction, FixedCellFont, IBM_CH_FILE, PlayState,
    RUNES_CH_FILE, Scene, TileGraphicsDepth, ViewOverlay, ViewOverlayKind, ViewOverlayMode,
    WorldPlane, dungeon_cell_index, parse_ch_font, world_cell_index,
};

use super::*;
use crate::gem::dungeon::DungeonInputs;
use crate::gem::knowledge::{KnowEvent, TownLink};
use crate::gem::layout::GemCanvas;
use crate::gem::raster::{GemImage, GemRender, MapArt, render_gem_map};
use crate::gem::shown::{GemViewState, TownInputs};

const C169: GemCanvas = GemCanvas { w: 512, h: 240 };
const C43: GemCanvas = GemCanvas { w: 320, h: 200 };
const BLACK: [u8; 4] = [0, 0, 0, 255];
const WHITE: [u8; 3] = MAP_PALETTE[15];

fn font(seed: u8) -> FixedCellFont {
    // A synthetic font whose "?" has a known ink box: rows 1..=6, columns
    // 2..=5; every other glyph is noise.
    let bytes = (0..CH_FONT_LEN)
        .map(|i| {
            let (code, row) = ((i / 8) as u8, (i % 8) as u8);
            if code == b'?' {
                return if (1..=6).contains(&row) { 0x3C } else { 0 };
            }
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

fn on() -> GemMapToggles {
    GemMapToggles {
        enhanced: true,
        ..GemMapToggles::default()
    }
}

fn off() -> GemMapToggles {
    GemMapToggles {
        labels: false,
        ..on()
    }
}

fn rgba(rgb: [u8; 3]) -> [u8; 4] {
    [rgb[0], rgb[1], rgb[2], 255]
}

fn gem_overlay(kind: ViewOverlayKind) -> Option<ViewOverlay> {
    Some(ViewOverlay {
        title: String::new(),
        text_map: String::new(),
        kind,
        mode: ViewOverlayMode::GemView,
    })
}

fn render(
    state: &PlayState,
    know: &GemKnowledge,
    t: &GemMapToggles,
    view: &GemViewState,
) -> GemImage {
    let mut out = GemImage::default();
    render_gem_map(state, know, &art(), t, view, &mut out);
    out
}

// ------------------------------------------------------------- outdoors

/// Britannia, grass, the party at `(x, y)`, a gem view up.
fn world(x: usize, y: usize) -> PlayState {
    let mut state = world_state(open_world_grid(), x, y);
    state.area = Area::World {
        plane: WorldPlane::Britannia,
    };
    state.active_objects[0].z = WorldPlane::Britannia.save_floor();
    state.active_view_overlay = gem_overlay(ViewOverlayKind::Surface);
    state
}

fn set_world(state: &mut PlayState, x: usize, y: usize, tile: u8) {
    state.grid[world_cell_index(x, y)] = tile;
}

fn object_at(state: &mut PlayState, x: usize, y: usize, tile: u8) {
    let z = state.active_objects[0].z;
    state.active_objects.push(ActiveObject {
        type_byte: tile,
        tile,
        x,
        y,
        z,
        ..ActiveObject::empty()
    });
}

/// Near Britain: the party at (86, 100), with Britain's town marker at
/// (81, 106), Lord British's castle wall (not a marker) at (86, 107), a
/// sign at (76, 113), a nameless hut marker at (100, 95), a live gate at
/// (96, 102) and a collapsed entrance in Despise's column.
fn near_britain() -> PlayState {
    let mut state = world(86, 100);
    set_world(&mut state, 81, 106, 0x14);
    set_world(&mut state, 86, 107, 0x3E);
    set_world(&mut state, 76, 113, 0xA4);
    set_world(&mut state, 100, 95, 0x10);
    set_world(&mut state, 96, 102, MOONGATE_TILE);
    set_world(&mut state, 91, 80, COLLAPSED_ENTRANCE_TILE);
    state
}

fn open(state: &PlayState, canvas: GemCanvas) -> GemViewState {
    GemViewState::open(state, canvas, 100)
}

fn outdoor_of(view: &GemViewState) -> &OutdoorShown {
    match &view.shown {
        GemShown::Outdoor(shown) => shown,
        other => panic!("an outdoor view, got {other:?}"),
    }
}

/// The tiles each category's "?" may sit on (10.2.7 table).
fn kind_tiles(category: FeatureCategory, tile: u8, glyphs: GlyphPolicy) -> bool {
    match category {
        FeatureCategory::Town
        | FeatureCategory::Castle
        | FeatureCategory::Dwelling
        | FeatureCategory::Dungeon
        | FeatureCategory::Shrine
        | FeatureCategory::OtherPlace => is_place_marker(tile),
        FeatureCategory::Moongate => tile == MOONGATE_TILE,
        FeatureCategory::Sign => SIGN_TILES.contains(&tile),
        FeatureCategory::Shop => SHOP_SIGN_TILES.contains(&tile),
        FeatureCategory::TownLink => (0xC4..=0xC9).contains(&tile),
        FeatureCategory::Exit => foot_terrain_accepts(tile),
        FeatureCategory::Ladder => (0x10..=0x3F).contains(&tile),
        FeatureCategory::Fountain => tile >> 4 == 0x5,
        FeatureCategory::Chest => tile >> 4 == 0x4,
        FeatureCategory::Trap => {
            matches!(tile, 0x60 | 0x68) || (glyphs == GlyphPolicy::GemFaithful && tile >> 4 == 6)
        }
    }
}

fn at(features: &[GemFeature], cell: FeatureCell) -> Vec<GemFeature> {
    features
        .iter()
        .filter(|feature| feature.cell == cell)
        .copied()
        .collect()
}

#[test]
fn outdoor_features_come_from_drawn_tiles_and_the_gazetteer_names_them() {
    let state = near_britain();
    let view = open(&state, C169);
    let shown = outdoor_of(&view);
    let features = gem_features(&view.shown, &GemKnowledge::new());
    let cell = |x, y| {
        let (dx, dy) = shown.offset(x, y).unwrap();
        FeatureCell::Outdoor { dx, dy }
    };
    // Britain's marker: one unknown place.
    let britain = at(&features, cell(81, 106));
    assert_eq!(britain.len(), 1);
    assert_eq!(britain[0].key(), Some(FeatureKey::Place { scene: 2 }));
    assert!(britain[0].drawn && !britain[0].known());
    // Lord British's castle wall is no marker: nothing until entered.
    assert!(at(&features, cell(86, 107)).is_empty());
    // The hut marker at no gazetteer cell: a "?" with no identity.
    let hut = at(&features, cell(100, 95));
    assert_eq!(hut.len(), 1);
    assert_eq!(hut[0].key(), None);
    // The live gate, the sign, the collapsed entrance (Despise by column).
    assert_eq!(
        at(&features, cell(96, 102))[0].key(),
        Some(FeatureKey::Gate(GateCell {
            plane: 0,
            x: 96,
            y: 102
        }))
    );
    assert_eq!(
        at(&features, cell(76, 113))[0].category,
        FeatureCategory::Sign
    );
    assert_eq!(
        at(&features, cell(91, 80))[0].key(),
        Some(FeatureKey::Entrance { record: 1 })
    );
    // Nothing anywhere else: grass carries nothing.
    assert_eq!(features.len(), 5);

    // Entering names them; the castle appears where its wall is drawn.
    let mut know = GemKnowledge::new();
    for event in [
        KnowEvent::PlaceEntered { scene: 2 },
        KnowEvent::PlaceEntered { scene: 17 },
        KnowEvent::EntranceNamed { record: 1 },
    ] {
        know.apply(&event);
    }
    let features = gem_features(&view.shown, &know);
    let britain = at(&features, cell(81, 106))[0];
    assert_eq!(britain.label(), Some("Britain"));
    assert_eq!(britain.category(), Some(FeatureCategory::Town));
    assert_eq!(britain.ring_colour(), MAP_PALETTE[15]);
    let castle = at(&features, cell(86, 107));
    assert_eq!(castle.len(), 1);
    assert_eq!(castle[0].label(), Some("Lord British's Castle"));
    assert_eq!(castle[0].category(), Some(FeatureCategory::Castle));
    assert!(!castle[0].drawn);
    assert_eq!(at(&features, cell(91, 80))[0].label(), Some("Despise"));
}

#[test]
fn every_question_mark_sits_on_a_drawn_tile_of_its_kind() {
    let state = near_britain();
    for canvas in [C169, C43] {
        let view = open(&state, canvas);
        let shown = outdoor_of(&view);
        for feature in gem_features(&view.shown, &GemKnowledge::new()) {
            assert!(!feature.known());
            let FeatureCell::Outdoor { dx, dy } = feature.cell else {
                panic!("an outdoor cell");
            };
            let tile = shown.tile(dx, dy).expect("a shown cell");
            assert!(feature.drawn);
            assert!(
                kind_tiles(feature.category, tile, GlyphPolicy::Perceived),
                "{feature:?} on {tile:#04x}"
            );
        }
    }
}

#[test]
fn nothing_undiscovered_has_a_colour_or_a_label() {
    let state = near_britain();
    let view = open(&state, C169);
    let know = GemKnowledge::new();
    for feature in gem_features(&view.shown, &know) {
        assert_eq!(feature.category(), None);
        assert_eq!(feature.label(), None);
        assert_eq!(feature.ring_colour(), NEUTRAL_GREY);
    }
    // In the picture: the marks over the plain map are only the tags'
    // grey, black and white.
    let marked = render(&state, &know, &on(), &view);
    let plain = render(&state, &know, &off(), &view);
    let mut changed = 0;
    for (a, b) in marked.rgba.chunks(4).zip(plain.rgba.chunks(4)) {
        if a != b {
            changed += 1;
            let a = [a[0], a[1], a[2], a[3]];
            assert!(
                [rgba(NEUTRAL_GREY), BLACK, rgba(WHITE)].contains(&a),
                "{a:?}"
            );
        }
    }
    assert!(changed > 0);
}

#[test]
fn the_question_tag_is_twelve_pixels_grey_ringed_with_cut_corners() {
    let state = near_britain();
    let view = open(&state, C169);
    let shown = outdoor_of(&view);
    let know = GemKnowledge::new();
    let out = render(&state, &know, &on(), &view);
    let plain = render(&state, &know, &off(), &view);
    let (dx, dy) = shown.offset(76, 113).unwrap();
    let (x0, y0, w, h) = shown.layout.cell_rect(dx, dy);
    let (tx, ty) = (x0 + w as i32 / 2 - 6, y0 + h as i32 / 2 - 6);
    let px = |img: &GemImage, i: i32, j: i32| img.pixel((tx + i) as u32, (ty + j) as u32).unwrap();
    // Cut corners: the map shows through.
    for (i, j) in [(0, 0), (11, 0), (0, 11), (11, 11)] {
        assert_eq!(px(&out, i, j), px(&plain, i, j));
    }
    for k in 1..11 {
        for (i, j) in [(k, 0), (k, 11), (0, k), (11, k)] {
            assert_eq!(px(&out, i, j), rgba(NEUTRAL_GREY), "({i}, {j})");
        }
    }
    for (i, j) in [(1, 1), (10, 1), (1, 10), (10, 10)] {
        assert_eq!(px(&out, i, j), rgba(NEUTRAL_GREY));
    }
    // The synthetic "?" ink is 4 x 6: centred, columns 4..=7, rows 3..=8.
    for j in 1..11 {
        for i in 1..11 {
            if (i == 1 || i == 10) && (j == 1 || j == 10) {
                continue;
            }
            let ink = (4..=7).contains(&i) && (3..=8).contains(&j);
            let want = if ink { rgba(WHITE) } else { BLACK };
            assert_eq!(px(&out, i, j), want, "({i}, {j})");
        }
    }
    // Outside the tag: nothing changed.
    assert_eq!(px(&out, -1, 5), px(&plain, -1, 5));
    assert_eq!(px(&out, 12, 5), px(&plain, 12, 5));
}

#[test]
fn a_discovered_feature_gets_its_category_ring_with_black_outside() {
    let state = near_britain();
    let view = open(&state, C169);
    let shown = outdoor_of(&view);
    let mut know = GemKnowledge::new();
    know.apply(&KnowEvent::SignRead(SignCell {
        scene: SCENE_OVERWORLD,
        floor: 0,
        x: 76,
        y: 113,
    }));
    know.apply(&KnowEvent::PlaceEntered { scene: 2 });
    let out = render(&state, &know, &on(), &view);
    for ((x, y), colour) in [((76, 113), MAP_PALETTE[7]), ((81, 106), MAP_PALETTE[15])] {
        let (dx, dy) = shown.offset(x, y).unwrap();
        let (x0, y0, w, _) = shown.layout.cell_rect(dx, dy);
        let x1 = x0 + w as i32;
        assert_eq!(
            out.pixel((x0 - 1) as u32, (y0 - 1) as u32),
            Some(rgba(colour))
        );
        assert_eq!(out.pixel(x1 as u32, y0 as u32), Some(rgba(colour)));
        assert_eq!(out.pixel((x0 - 2) as u32, (y0 + 1) as u32), Some(BLACK));
        assert_eq!(out.pixel((x1 + 1) as u32, y0 as u32), Some(BLACK));
    }
}

#[test]
fn a_moongate_is_marked_only_while_up_or_once_used() {
    // L3. Up at (96, 100); sunk at (104, 100), where the party once used
    // it; grass at (108, 100), where a gate rises some other night.
    let mut state = world(100, 100);
    set_world(&mut state, 96, 100, MOONGATE_TILE);
    let mut know = GemKnowledge::new();
    know.apply(&KnowEvent::GateUsed(GateCell {
        plane: 0,
        x: 104,
        y: 100,
    }));
    // A gate used on the other plane says nothing about this one.
    know.apply(&KnowEvent::GateUsed(GateCell {
        plane: 1,
        x: 108,
        y: 100,
    }));
    let view = open(&state, C169);
    let features = gem_features(&view.shown, &know);
    let gates = features
        .iter()
        .filter(|f| f.category == FeatureCategory::Moongate)
        .count();
    assert_eq!(gates, 2);
    let up = at(&features, FeatureCell::Outdoor { dx: -4, dy: 0 });
    assert!(up[0].drawn && !up[0].known());
    let sunk = at(&features, FeatureCell::Outdoor { dx: 4, dy: 0 });
    assert!(!sunk[0].drawn && sunk[0].known());
    assert_eq!(sunk[0].label(), Some("Moongate"));
    assert!(at(&features, FeatureCell::Outdoor { dx: 8, dy: 0 }).is_empty());
}

#[test]
fn hidden_objects_get_no_mark() {
    // A buried moonstone, a shard, the Amulet and the arms cache are
    // objects or hidden items on grass: the map draws terrain only, so
    // none of them is a feature, whatever tile it would show.
    let mut state = world(100, 100);
    for (i, tile) in [0x19, 0x10, 0x14, 0xDC, 0xA4, 0xDF].into_iter().enumerate() {
        object_at(&mut state, 102 + i, 101, tile);
    }
    let view = open(&state, C169);
    let know = GemKnowledge::new();
    assert!(gem_features(&view.shown, &know).is_empty());
    assert_eq!(
        render(&state, &know, &on(), &view),
        render(&state, &know, &off(), &view)
    );
}

#[test]
fn an_object_over_an_entrance_raises_no_question_mark() {
    // L9: the gazetteer gives a marker its name, never its existence.
    // Britain's cell and the Shrine of Compassion's with grass drawn and
    // an object standing on each: no feature, though the gazetteer has
    // places there.
    let mut state = world(100, 100);
    object_at(&mut state, 81, 106, 0x14);
    object_at(&mut state, 128, 92, 0x19);
    let view = open(&state, C169);
    assert!(gem_features(&view.shown, &GemKnowledge::new()).is_empty());
    // With the marker drawn, an object over it changes nothing: the "?"
    // highlights the marker the picture shows.
    set_world(&mut state, 81, 106, 0x14);
    let view = open(&state, C169);
    let features = gem_features(&view.shown, &GemKnowledge::new());
    assert_eq!(features.len(), 1);
    assert!(features[0].drawn);
}

#[test]
fn the_codex_is_a_place() {
    // L8: the Codex's marker carries a "?"; once entered it is a place in
    // the other-place category, named, never a moongate.
    let mut state = world(230, 230);
    set_world(&mut state, 233, 233, 0x11);
    let view = open(&state, C169);
    let cell = FeatureCell::Outdoor { dx: 3, dy: 3 };
    let unknown = at(&gem_features(&view.shown, &GemKnowledge::new()), cell);
    assert_eq!(unknown.len(), 1);
    assert_eq!(unknown[0].key(), Some(FeatureKey::Codex));
    assert_eq!(unknown[0].category(), None);
    let mut know = GemKnowledge::new();
    know.apply(&KnowEvent::CodexEntered);
    let known = at(&gem_features(&view.shown, &know), cell)[0];
    assert_eq!(known.category(), Some(FeatureCategory::OtherPlace));
    assert_eq!(known.label(), Some("Codex"));
    assert_eq!(known.ring_colour(), MAP_PALETTE[13]);
}

#[test]
fn shrines_are_named_by_virtue_once_entered() {
    let mut state = world(128, 95);
    set_world(&mut state, 128, 92, 0x19);
    let view = open(&state, C169);
    let mut know = GemKnowledge::new();
    know.apply(&KnowEvent::ShrineEntered { virtue: 1 });
    let shrine = at(
        &gem_features(&view.shown, &know),
        FeatureCell::Outdoor { dx: 0, dy: -3 },
    )[0];
    assert_eq!(shrine.label(), Some("Compassion"));
    assert_eq!(shrine.ring_colour(), MAP_PALETTE[11]);
}

/// The features as `(cell and kind, known)`, sorted, for comparing two
/// records.
fn known_map(features: &[GemFeature]) -> Vec<(String, bool)> {
    let mut out = features
        .iter()
        .map(|f| (format!("{:?}/{:?}", f.cell, f.category), f.known()))
        .collect::<Vec<_>>();
    out.sort();
    out
}

fn changed(before: &[GemFeature], after: &[GemFeature]) -> usize {
    let (a, b) = (known_map(before), known_map(after));
    assert_eq!(a.len(), b.len(), "a record bit never adds a drawn feature");
    a.iter().zip(&b).filter(|(x, y)| x != y).count()
}

#[test]
fn one_record_bit_changes_one_mark_outdoors() {
    let state = near_britain();
    let view = open(&state, C169);
    let base = GemKnowledge::new();
    let before = gem_features(&view.shown, &base);
    for event in [
        KnowEvent::PlaceEntered { scene: 2 },
        KnowEvent::EntranceNamed { record: 1 },
        KnowEvent::GateUsed(GateCell {
            plane: 0,
            x: 96,
            y: 102,
        }),
        KnowEvent::SignRead(SignCell {
            scene: SCENE_OVERWORLD,
            floor: 0,
            x: 76,
            y: 113,
        }),
    ] {
        let mut know = base.clone();
        know.apply(&event);
        let after = gem_features(&view.shown, &know);
        assert_eq!(changed(&before, &after), 1, "{event:?}");
    }
    // A bit for something off the map changes nothing.
    let mut know = base.clone();
    know.apply(&KnowEvent::PlaceEntered { scene: 1 });
    assert_eq!(changed(&before, &gem_features(&view.shown, &know)), 0);
}

#[test]
fn with_labels_off_nothing_is_drawn() {
    let state = near_britain();
    let view = open(&state, C169);
    let mut know = GemKnowledge::new();
    for scene in 1..=32 {
        know.apply(&KnowEvent::PlaceEntered { scene });
    }
    let without = render(&state, &GemKnowledge::new(), &off(), &view);
    assert_eq!(render(&state, &know, &off(), &view), without);
    assert_ne!(render(&state, &know, &on(), &view), without);
}

// ----------------------------------------------------------------- towns

fn britain() -> Scene {
    Scene::from_key("TOWNE:1").unwrap()
}

/// Britain floor 0: a wall round a cobbled floor, a south gate at x
/// 14..=17, a hidden wall in the east edge at (31, 10), and on the floor a
/// sign, the seven signed shops and the two unmapped ones, a two-cell
/// stairway and a ladder. The party at (15, 17).
fn town() -> PlayState {
    let mut grid = vec![0x44; 1024];
    for i in 0..32 {
        for (x, y) in [(i, 0), (i, 31), (0, i), (31, i)] {
            grid[y * 32 + x] = 0x4D;
        }
    }
    for x in 14..=17 {
        grid[31 * 32 + x] = 0x44;
    }
    grid[10 * 32 + 31] = 0x4E;
    grid[5 * 32 + 5] = 0xA4;
    for (i, tile) in SHOP_SIGN_TILES.into_iter().enumerate() {
        grid[3 * 32 + 3 + 2 * i] = tile;
    }
    grid[8 * 32 + 20] = 0xC4;
    grid[9 * 32 + 20] = 0xC4; // the stairway's second cell
    grid[9 * 32 + 22] = 0xC8;
    let mut state = test_state(grid, 15, 17);
    state.area = Area::Town {
        scene: britain(),
        floor: 0,
    };
    state.active_view_overlay = gem_overlay(ViewOverlayKind::Surface);
    state
}

fn town_cell(fx: i32, fy: i32) -> FeatureCell {
    FeatureCell::Town { fx, fy, exit: None }
}

#[test]
fn town_features_and_their_question_marks() {
    let state = town();
    let view = open(&state, C169);
    let GemShown::Town(shown) = &view.shown else {
        panic!("a town view");
    };
    let features = gem_features(&view.shown, &GemKnowledge::new());
    // A sign, nine shop signs, two links and the south gate.
    assert_eq!(features.len(), 13, "{features:?}");
    for feature in &features {
        assert!(!feature.known() && feature.label().is_none());
        let FeatureCell::Town { fx, fy, .. } = feature.cell else {
            panic!("a town cell");
        };
        let tile = shown.tile(fx, fy).expect("a shown cell");
        assert!(kind_tiles(feature.category, tile, GlyphPolicy::Perceived));
    }
    let exits = features
        .iter()
        .filter(|f| f.exit_side().is_some())
        .collect::<Vec<_>>();
    assert_eq!(exits.len(), 1);
    assert_eq!(
        exits[0].cell,
        FeatureCell::Town {
            fx: 15,
            fy: 31,
            exit: Some(EdgeSide::South)
        }
    );
}

#[test]
fn shops_are_named_by_trade_and_unmapped_signs_stay_question_marks() {
    let state = town();
    let view = open(&state, C169);
    let scene = britain().byte;
    let mut know = GemKnowledge::new();
    for trade in crate::gem::knowledge::TRADES {
        know.apply(&KnowEvent::ShopOpened {
            scene,
            trade,
            keeper: None,
        });
    }
    let features = gem_features(&view.shown, &know);
    for (i, tile) in SHOP_SIGN_TILES.into_iter().enumerate() {
        let shop = at(&features, town_cell(3 + 2 * i as i32, 3));
        assert_eq!(shop.len(), 1);
        match Trade::from_sign_tile(tile) {
            Some(trade) => {
                assert_eq!(shop[0].label(), Some(trade_label(trade)));
                assert_eq!(shop[0].ring_colour(), MAP_PALETTE[11]);
            }
            None => {
                assert!(matches!(tile, 0xF0 | 0xF1));
                assert!(!shop[0].known());
            }
        }
    }
    // Tavern and Sage with no keeper cell recorded: nowhere.
    assert!(
        features
            .iter()
            .all(|f| !matches!(f.label(), Some("Tavern" | "Sage")))
    );
}

#[test]
fn the_tavern_appears_only_at_its_keepers_cell() {
    // L4: no sign tile, no "?"; a label only where the keeper stood.
    let state = town();
    let view = open(&state, C169);
    let scene = britain().byte;
    let before = gem_features(&view.shown, &GemKnowledge::new());
    let mut know = GemKnowledge::new();
    know.apply(&KnowEvent::ShopOpened {
        scene,
        trade: Trade::Tavern,
        keeper: Some((0, 2, 3)),
    });
    know.apply(&KnowEvent::ShopOpened {
        scene,
        trade: Trade::Sage,
        keeper: Some((1, 9, 9)), // another floor
    });
    let after = gem_features(&view.shown, &know);
    assert_eq!(after.len(), before.len() + 1);
    let tavern = at(&after, town_cell(2, 3));
    assert_eq!(tavern.len(), 1);
    assert_eq!(tavern[0].label(), Some("Tavern"));
    assert!(!tavern[0].drawn);
}

#[test]
fn town_links_take_their_word_from_use() {
    let state = town();
    let view = open(&state, C169);
    let scene = britain().byte;
    let mut know = GemKnowledge::new();
    for (x, y, dir) in [(20, 8, LinkDir::Up), (22, 9, LinkDir::Down)] {
        know.apply(&KnowEvent::TownLinkUsed(TownLink {
            scene,
            floor: 0,
            x,
            y,
            dir,
        }));
    }
    let features = gem_features(&view.shown, &know);
    assert_eq!(
        at(&features, town_cell(20, 8))[0].label(),
        Some("Stairs up")
    );
    // The stairway is one feature, at its first cell.
    assert!(at(&features, town_cell(20, 9)).is_empty());
    assert_eq!(
        at(&features, town_cell(22, 9))[0].label(),
        Some("Ladder down")
    );
}

#[test]
fn a_stairway_is_known_from_any_of_its_cells() {
    let state = town();
    let view = open(&state, C169);
    let before = gem_features(&view.shown, &GemKnowledge::new());
    let mut know = GemKnowledge::new();
    know.apply(&KnowEvent::TownLinkUsed(TownLink {
        scene: britain().byte,
        floor: 0,
        x: 20,
        y: 9,
        dir: LinkDir::Down,
    }));
    let after = gem_features(&view.shown, &know);
    assert_eq!(changed(&before, &after), 1);
    assert_eq!(at(&after, town_cell(20, 8))[0].label(), Some("Stairs down"));
}

#[test]
fn exits_follow_the_drawn_edge_and_a_hidden_wall_is_none() {
    let mut state = town();
    let view = open(&state, C169);
    let scene = britain().byte;
    let mut know = GemKnowledge::new();
    know.apply(&KnowEvent::TownExit(ExitRun {
        scene,
        side: EdgeSide::South,
        at: 16,
    }));
    let features = gem_features(&view.shown, &know);
    let exit = features.iter().find(|f| f.exit_side().is_some()).unwrap();
    assert!(exit.known());
    assert_eq!(exit.label(), Some("Exit"));
    // No exit on the east side: the hidden wall is not passable.
    assert!(
        features
            .iter()
            .all(|f| f.exit_side() != Some(EdgeSide::East))
    );
    // Land outside the west wall the party cannot walk to is no exit.
    for y in 20..=24 {
        state.grid[y * 32] = 0x05;
        state.grid[y * 32 + 1] = 0x4D;
    }
    let features = gem_features(&open(&state, C169).shown, &know);
    assert!(
        features
            .iter()
            .all(|f| f.exit_side() != Some(EdgeSide::West))
    );
    // Opened, the east cell is a one-cell run.
    state.grid[10 * 32 + 31] = 0x44;
    let view = open(&state, C169);
    let features = gem_features(&view.shown, &know);
    let east = FeatureCell::Town {
        fx: 31,
        fy: 10,
        exit: Some(EdgeSide::East),
    };
    assert!(features.iter().any(|f| f.cell == east));
}

#[test]
fn the_exit_arrow_and_its_question_mark_sit_outside_the_floor() {
    let state = town();
    let view = open(&state, C169);
    let GemShown::Town(shown) = &view.shown else {
        panic!("a town view");
    };
    let (x0, y0, w, h) = shown.layout.cell_rect(15, 32);
    let (cx, cy) = (x0 + w as i32 / 2, y0 + h as i32 / 2);
    let out = render(&state, &GemKnowledge::new(), &on(), &view);
    // Unknown: the "?" tag centred one cell below the gate.
    assert_eq!(
        out.pixel((cx - 5) as u32, (cy - 6) as u32),
        Some(rgba(NEUTRAL_GREY))
    );
    let mut know = GemKnowledge::new();
    know.apply(&KnowEvent::TownExit(ExitRun {
        scene: britain().byte,
        side: EdgeSide::South,
        at: 15,
    }));
    let out = render(&state, &know, &on(), &view);
    // Known: the white arrow, no grey.
    let tag = (cx - 6..cx + 6)
        .flat_map(|x| (cy - 6..cy + 6).map(move |y| (x, y)))
        .filter_map(|(x, y)| out.pixel(x as u32, y as u32))
        .collect::<Vec<_>>();
    assert!(tag.contains(&rgba(WHITE)));
    assert!(!tag.contains(&rgba(NEUTRAL_GREY)));
}

#[test]
fn one_record_bit_changes_one_mark_in_a_town() {
    let state = town();
    let view = open(&state, C169);
    let scene = britain().byte;
    let before = gem_features(&view.shown, &GemKnowledge::new());
    for event in [
        KnowEvent::SignRead(SignCell {
            scene,
            floor: 0,
            x: 5,
            y: 5,
        }),
        KnowEvent::ShopOpened {
            scene,
            trade: Trade::Healer,
            keeper: None,
        },
        KnowEvent::TownLinkUsed(TownLink {
            scene,
            floor: 0,
            x: 20,
            y: 8,
            dir: LinkDir::Up,
        }),
        KnowEvent::TownExit(ExitRun {
            scene,
            side: EdgeSide::South,
            at: 14,
        }),
    ] {
        let mut know = GemKnowledge::new();
        know.apply(&event);
        assert_eq!(
            changed(&before, &gem_features(&view.shown, &know)),
            1,
            "{event:?}"
        );
    }
}

#[test]
fn people_on_signs_and_shops_change_nothing() {
    // The floor is terrain only: a townsperson on the sign and a creature
    // on a shop sign leave the "?" where the picture draws them, and an
    // object showing a shop sign makes none.
    let mut state = town();
    let before = gem_features(&open(&state, C169).shown, &GemKnowledge::new());
    object_at(&mut state, 5, 5, 0x60);
    object_at(&mut state, 3, 3, 0x90);
    object_at(&mut state, 10, 10, 0xF2);
    let after = gem_features(&open(&state, C169).shown, &GemKnowledge::new());
    assert_eq!(before, after);
}

// -------------------------------------------------------------- dungeons

fn set_dungeon(state: &mut PlayState, level: u8, x: usize, y: usize, byte: u8) {
    state.grid[dungeon_cell_index(level, x, y)] = byte;
}

/// Every level all wall except an open level `level` (the party at (1, 1)
/// facing north) holding a ladder up and one down, a fountain, a closed
/// chest, an open chest, a visible pit, a hidden pit, a bomb trap and a
/// secret door; a dungeon gem view up.
fn dungeon(level: u8) -> PlayState {
    let mut state = dungeon_state(open_dungeon_record(), level, 1, 1);
    for lvl in 0..8 {
        for y in 0..8 {
            for x in 0..8 {
                let byte = if lvl == level { 0x00 } else { 0xB0 };
                set_dungeon(&mut state, lvl, x, y, byte);
            }
        }
    }
    for (x, y, byte) in [
        (2, 1, 0x10),
        (3, 1, 0x20),
        (4, 1, 0x50),
        (5, 1, 0x40),
        (6, 1, 0x70),
        (2, 3, 0x60),
        (3, 3, 0x61),
        (4, 3, 0x62),
        (5, 3, 0xD0),
    ] {
        set_dungeon(&mut state, level, x, y, byte);
    }
    state.player.facing = Direction::North;
    state.active_view_overlay = gem_overlay(ViewOverlayKind::Dungeon { level });
    state
}

fn record_of(state: &PlayState) -> u8 {
    match state.area {
        Area::Dungeon { scene, .. } => scene.record as u8,
        _ => unreachable!("a dungeon"),
    }
}

fn dungeon_view(state: &PlayState, know: &GemKnowledge, glyphs: GlyphPolicy) -> GemViewState {
    GemViewState::open_all(
        state,
        C169,
        100,
        TownInputs::default(),
        DungeonInputs {
            know: Some(know),
            glyphs,
        },
    )
}

fn dcell(level: u8, x: u8, y: u8) -> FeatureCell {
    FeatureCell::Dungeon { level, x, y }
}

fn mark(record: u8, level: u8, x: u8, y: u8, kind: MarkKind) -> KnowEvent {
    KnowEvent::DungeonMarked(DungeonMark {
        record,
        level,
        x,
        y,
        kind,
    })
}

#[test]
fn dungeon_features_follow_the_drawn_glyphs() {
    let state = dungeon(2);
    let know = GemKnowledge::new();
    let view = dungeon_view(&state, &know, GlyphPolicy::Perceived);
    let GemShown::Dungeon(shown) = &view.shown else {
        panic!("a dungeon view");
    };
    let features = gem_features(&view.shown, &know);
    let kinds = |x, y| {
        at(&features, dcell(2, x, y))
            .iter()
            .map(|f| f.category)
            .collect::<Vec<_>>()
    };
    assert_eq!(kinds(2, 1), [FeatureCategory::Ladder]);
    assert_eq!(kinds(3, 1), [FeatureCategory::Ladder]);
    assert_eq!(kinds(4, 1), [FeatureCategory::Fountain]);
    assert_eq!(kinds(5, 1), [FeatureCategory::Chest]);
    assert_eq!(kinds(6, 1), []); // the open chest draws nothing
    assert_eq!(kinds(2, 3), [FeatureCategory::Trap]);
    assert_eq!(kinds(3, 3), []); // a hidden pit draws as passage
    assert_eq!(kinds(4, 3), []); // the bomb trap too
    assert_eq!(kinds(5, 3), []); // a secret door is never a feature
    assert_eq!(features.len(), 5);
    for feature in &features {
        let FeatureCell::Dungeon { level, x, y } = feature.cell else {
            panic!("a dungeon cell");
        };
        let cell = shown.cell(level, x, y).expect("a shown cell");
        assert!(kind_tiles(
            feature.category,
            cell.drawn,
            GlyphPolicy::Perceived
        ));
        assert!(feature.category().is_none());
    }
    // Under gem-faithful, the trap glyphs carry a "?" too.
    let view = dungeon_view(&state, &know, GlyphPolicy::GemFaithful);
    let features = gem_features(&view.shown, &know);
    assert_eq!(at(&features, dcell(2, 3, 3)).len(), 1);
    assert_eq!(at(&features, dcell(2, 4, 3)).len(), 1);
    assert!(at(&features, dcell(2, 5, 3)).is_empty());
}

#[test]
fn dungeon_labels_and_found_features() {
    let state = dungeon(2);
    let record = record_of(&state);
    let mut know = GemKnowledge::new();
    for (x, y, kind) in [
        (2, 1, MarkKind::Ladder),
        (3, 1, MarkKind::Ladder),
        (4, 1, MarkKind::Fountain),
        (6, 1, MarkKind::Chest),
        (3, 3, MarkKind::Trap),
    ] {
        know.apply(&mark(record, 2, x, y, kind));
    }
    let view = dungeon_view(&state, &know, GlyphPolicy::Perceived);
    let features = gem_features(&view.shown, &know);
    let label = |x, y| at(&features, dcell(2, x, y))[0].label();
    assert_eq!(label(2, 1), Some("Up"));
    assert_eq!(label(3, 1), Some("Down"));
    assert_eq!(label(4, 1), Some("Fountain"));
    // The opened chest keeps its label though it draws nothing now.
    assert_eq!(label(6, 1), Some("Chest"));
    assert_eq!(at(&features, dcell(2, 6, 1))[0].ring_colour(), CHEST_BROWN);
    // The found hidden pit draws as a pit and is a known trap.
    assert_eq!(label(3, 3), Some("Trap"));
    // Level 1's way up and the deepest level's way down are exits.
    assert_eq!(ladder_label(0x10, 0), "Exit up");
    assert_eq!(ladder_label(0x20, 7), "Exit down");
    assert_eq!(ladder_label(0x30, 3), "Up/Down");
}

#[test]
fn dungeon_question_marks_sit_on_the_top_right_corner() {
    let state = dungeon(2);
    let know = GemKnowledge::new();
    let view = dungeon_view(&state, &know, GlyphPolicy::Perceived);
    let GemShown::Dungeon(shown) = &view.shown else {
        panic!("a dungeon view");
    };
    let layout = crate::gem::layout::DungeonLayout::new(C169, true).unwrap();
    let panels = crate::gem::raster::dungeon_panel_levels(shown, &layout, 0);
    let index = panels.iter().find(|(_, l)| *l == 2).unwrap().0;
    let (px, py) = layout.panel_origin(index);
    let c = layout.cell as i32;
    let out = render(&state, &know, &on(), &view);
    let plain = render(&state, &know, &off(), &view);
    // The fountain at (4, 1): its tag's top-left 9 px in from the cell's
    // top-right corner and 3 px above it.
    let (x0, y0) = (px + 4 * c, py + c);
    let (tx, ty) = (x0 + c - 12 + 3, y0 - 3);
    assert_eq!(
        out.pixel((tx + 1) as u32, ty as u32),
        Some(rgba(NEUTRAL_GREY))
    );
    assert_eq!(
        out.pixel(tx as u32, ty as u32),
        plain.pixel(tx as u32, ty as u32)
    );
    // Known dungeon features get no ring before their label (M3-16).
    let mut know = GemKnowledge::new();
    know.apply(&mark(shown.record, 2, 4, 1, MarkKind::Fountain));
    let view = dungeon_view(&state, &know, GlyphPolicy::Perceived);
    let out = render(&state, &know, &on(), &view);
    let plain = render(&state, &know, &off(), &view);
    // Where its ring would be, right and below (the ladder's "?" to its
    // left overlaps the left edge).
    let ring = (y0..y0 + c + 2)
        .flat_map(|y| [(x0 + c, y), (x0 + c + 1, y)])
        .chain((x0..x0 + c + 2).flat_map(|x| [(x, y0 + c), (x, y0 + c + 1)]));
    for (x, y) in ring {
        assert_eq!(
            out.pixel(x as u32, y as u32),
            plain.pixel(x as u32, y as u32)
        );
    }
}

#[test]
fn a_black_panel_carries_a_question_mark_and_no_underworld_name() {
    // The deepest level with a ladder down: the Underworld slot is a
    // black panel with a "?", untitled until the party has been there.
    let mut state = dungeon(7);
    set_dungeon(&mut state, 7, 1, 2, 0x20);
    let know = GemKnowledge::new();
    let view = dungeon_view(&state, &know, GlyphPolicy::Perceived);
    let GemShown::Dungeon(shown) = &view.shown else {
        panic!("a dungeon view");
    };
    assert_eq!(
        shown.slot(8),
        crate::gem::dungeon::DungeonSlot::Underworld { named: false }
    );
    assert_eq!(shown.title(8), None);
    let layout = crate::gem::layout::DungeonLayout::new(C169, true).unwrap();
    let panels = crate::gem::raster::dungeon_panel_levels(shown, &layout, 0);
    let index = panels.iter().find(|(_, l)| *l == 8).unwrap().0;
    let (px, py) = layout.panel_origin(index);
    let p = layout.panel_px() as i32;
    let mut out = GemImage::default();
    assert_eq!(
        render_gem_map(&state, &know, &art(), &on(), &view, &mut out),
        GemRender::Dungeon
    );
    assert_eq!(
        out.pixel((px + p / 2 - 5) as u32, (py + p / 2 - 6) as u32),
        Some(rgba(NEUTRAL_GREY))
    );
    // With labels off, the panel stays plain black.
    let plain = render(&state, &know, &off(), &view);
    assert_eq!(
        plain.pixel((px + p / 2 - 5) as u32, (py + p / 2 - 6) as u32),
        Some(BLACK)
    );
    // No feature names the Underworld.
    for feature in gem_features(&view.shown, &know) {
        assert_ne!(feature.label(), Some("Underworld"));
    }
}

#[test]
fn one_record_bit_changes_one_mark_in_a_dungeon() {
    let state = dungeon(2);
    let record = record_of(&state);
    let base = GemKnowledge::new();
    let view = dungeon_view(&state, &base, GlyphPolicy::Perceived);
    let before = gem_features(&view.shown, &base);
    for (x, y, kind) in [
        (2, 1, MarkKind::Ladder),
        (4, 1, MarkKind::Fountain),
        (5, 1, MarkKind::Chest),
        (2, 3, MarkKind::Trap),
    ] {
        let mut know = base.clone();
        know.apply(&mark(record, 2, x, y, kind));
        let after = gem_features(&view.shown, &know);
        assert_eq!(changed(&before, &after), 1, "{kind:?}");
    }
}

#[test]
fn the_features_never_change_the_game() {
    let state = near_britain();
    let before = u5_runtime::enhancements::trajectory_digest(&state);
    let view = open(&state, C169);
    let _ = render(&state, &GemKnowledge::new(), &on(), &view);
    let _ = gem_features(&view.shown, &GemKnowledge::new());
    assert_eq!(u5_runtime::enhancements::trajectory_digest(&state), before);
}

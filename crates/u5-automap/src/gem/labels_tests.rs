//! M3-16 label placement tests (10.2.7 "Placement" and "Order", L1, L2,
//! D10.4): no two labels overlap or touch, every tag lies inside its
//! bounds and the canvas, clear of the party and of every other mark, and
//! no leader crosses a tag or a mark, on fixed and randomised sets; the
//! order puts every "?" after the discovered labels by distance only, so
//! permuting the kinds of the "?" features changes nothing; the 4:3 town
//! sides show rings without names; the same input gives the same layout.

use u5_runtime::enhancements::GemMapToggles;
use u5_runtime::test_fixtures::{
    open_grid, open_world_grid, synthetic_tile_atlas, test_state, world_state,
};
use u5_runtime::{
    Area, PlayState, PlayTarget, Scene, TileGraphicsDepth, ViewOverlay, ViewOverlayKind,
    ViewOverlayMode, WORLD_SIDE, WorldPlane, published_world_location_entries, world_cell_index,
};

use super::*;
use crate::gem::features::{COLLAPSED_ENTRANCE_TILE, MOONGATE_TILE, gem_features};
use crate::gem::knowledge::{GemKnowledge, KnowEvent, SIDE_WINDOW_HEIGHT, SIDE_WINDOW_WIDTH};
use crate::gem::raster::{GemImage, MapArt, render_gem_map};
use crate::gem::shown::{GemViewState, TownInputs};

const C169: GemCanvas = GemCanvas { w: 512, h: 240 };
const C43: GemCanvas = GemCanvas { w: 320, h: 200 };
const C219: GemCanvas = GemCanvas { w: 688, h: 240 };

const WORDS: [&str; 8] = [
    "Sign",
    "Inn",
    "Moongate",
    "Britain",
    "Lord British's Castle",
    "Up",
    "Exit down",
    "Compassion",
];

/// A tiny deterministic generator for the randomised sets.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, n: u64) -> i32 {
        (self.next() % n) as i32
    }
}

fn item(x: i32, y: i32, size: i32, text: LabelText, rank: u8, distance: u32) -> LabelItem {
    LabelItem {
        cell: (x, y, x + size, y + size),
        text,
        rank,
        distance,
        colour: if text == LabelText::Question {
            NEUTRAL_GREY
        } else {
            [255, 255, 255]
        },
        own_mark: false,
        ring_known: true,
    }
}

/// The ink-free margin of a tag: a text tag's outer pixel is outline or
/// map, so its letters sit 1 px in; a "?" tag's ring is its edge.
fn ink(tag: Rect, question: bool) -> Rect {
    if question {
        tag
    } else {
        (tag.0 + 1, tag.1 + 1, tag.2 - 1, tag.3 - 1)
    }
}

/// The gap between two boxes in pixels along the axis they are apart on
/// (0 when they overlap).
fn gap(a: Rect, b: Rect) -> i32 {
    let dx = (b.0 - a.2).max(a.0 - b.2);
    let dy = (b.1 - a.3).max(a.1 - b.3);
    dx.max(dy).max(0)
}

/// The invariants of one group's layout.
fn check_group(items: &[LabelItem], layout: &GroupLayout, bounds: Rect, keepout: &[Rect]) {
    assert_eq!(layout.placed.len(), items.len());
    let tags: Vec<(usize, Rect, bool)> = items
        .iter()
        .zip(&layout.placed)
        .enumerate()
        .filter_map(|(i, (item, p))| Some((i, p.tag?, item.unknown())))
        .collect();
    for &(i, tag, question) in &tags {
        let item = &items[i];
        // The right size and inside the bounds, a "?" 1 px in.
        assert_eq!(Some((tag.2 - tag.0, tag.3 - tag.1)), item.tag_size());
        let room = clearance(tag, question);
        assert!(
            room.0 >= bounds.0 && room.1 >= bounds.1 && room.2 <= bounds.2 && room.3 <= bounds.3,
            "{tag:?} outside {bounds:?}"
        );
        // Clear of the keep-out boxes and every other feature's mark; off
        // its own mark unless it is a "?" on its cell.
        for &k in keepout {
            assert!(!overlaps(room, k), "{tag:?} on keep-out {k:?}");
        }
        for (j, other) in items.iter().enumerate() {
            if j != i {
                assert!(!overlaps(room, other.mark_box()), "{tag:?} on mark {j}");
            }
        }
        if layout.placed[i].how != Some(Placement::OnCell) {
            assert!(!overlaps(tag, item.mark_box()));
        } else {
            assert!(question);
        }
        // No two labels' ink overlap or touch: at least 1 px of outline or
        // black between them.
        for &(j, other, q) in &tags {
            if j > i {
                assert!(!overlaps(tag, other), "{tag:?} overlaps {other:?}");
                assert!(gap(ink(tag, question), ink(other, q)) >= 2);
            }
        }
        // A leader crosses no other tag and no other mark.
        if let Some(Placement::Leader { end }) = layout.placed[i].how {
            let points = line_points(leader_start(centre(item.cell)), end);
            for (x, y) in points {
                if overlaps((x, y, x + 1, y + 1), item.mark_box())
                    || overlaps((x, y, x + 1, y + 1), tag)
                {
                    continue;
                }
                // The 3 px black line too.
                let p = (x - 1, y - 1, x + 2, y + 2);
                for &(j, other, _) in &tags {
                    if j != i {
                        assert!(!overlaps(p, other), "leader {i} crosses tag {j}");
                    }
                }
                for (j, other) in items.iter().enumerate() {
                    if j != i {
                        assert!(
                            !overlaps(p, other.mark_box()),
                            "leader {i} crosses mark {j}"
                        );
                    }
                }
            }
        }
    }
    let missing = items
        .iter()
        .zip(&layout.placed)
        .filter(|(item, p)| item.tag_size().is_some() && p.tag.is_none())
        .count();
    assert_eq!(missing, layout.unlabelled);
}

/// A crowd: a 6 x 4 block of 4 px cells, half named, half "?", round a
/// party box, near the canvas's top edge.
fn crowd() -> (Vec<LabelItem>, Rect, Rect) {
    let mut items = Vec::new();
    for j in 0..4 {
        for i in 0..6 {
            let (x, y) = (200 + 14 * i, 6 + 14 * j);
            let k = (j * 6 + i) as usize;
            let text = if k.is_multiple_of(2) {
                LabelText::Word(WORDS[k % WORDS.len()])
            } else {
                LabelText::Question
            };
            items.push(item(x, y, 4, text, (k % 7) as u8, (i * i + j * j) as u32));
        }
    }
    let party = (226, 30, 246, 50);
    (items, (0, 0, 512, 240), party)
}

#[test]
fn a_crowd_is_laid_out_without_overlap_inside_the_canvas_and_clear_of_the_party() {
    let (items, bounds, party) = crowd();
    let layout = place_labels(&items, TagAt::Centre, bounds, &[party]);
    check_group(&items, &layout, bounds, &[party]);
    // Most of the crowd is labelled, some by leader.
    assert!(layout.unlabelled * 2 < items.len(), "{}", layout.unlabelled);
    assert!(
        layout
            .placed
            .iter()
            .any(|p| matches!(p.how, Some(Placement::Leader { .. })))
    );
}

#[test]
fn every_tag_tries_its_cell_then_beside_then_a_leader() {
    // Alone, a "?" sits centred on its cell and a name right of its mark.
    let q = item(100, 100, 4, LabelText::Question, 0, 0);
    let layout = place_labels(&[q], TagAt::Centre, (0, 0, 512, 240), &[]);
    assert_eq!(layout.placed[0].how, Some(Placement::OnCell));
    assert_eq!(layout.placed[0].tag, Some((96, 96, 108, 108)));
    assert!(!layout.placed[0].ring, "a \"?\" on its cell has no ring");
    let n = item(100, 100, 4, LabelText::Word("Inn"), 2, 0);
    let layout = place_labels(&[n], TagAt::Centre, (0, 0, 512, 240), &[]);
    assert_eq!(layout.placed[0].how, Some(Placement::Beside));
    assert_eq!(layout.placed[0].tag, Some((106, 97, 132, 107)));
    assert!(layout.placed[0].ring);
    // On a dungeon cell the "?" sits on the top-right corner.
    let d = item(100, 100, 20, LabelText::Question, 0, 0);
    let layout = place_labels(&[d], TagAt::Corner, (0, 0, 512, 240), &[]);
    assert_eq!(layout.placed[0].tag, Some((111, 97, 123, 109)));
    // A known dungeon feature's label marks it: no ring.
    let mut k = item(100, 100, 20, LabelText::Word("Up"), 1, 0);
    k.ring_known = false;
    let layout = place_labels(&[k], TagAt::Corner, (0, 0, 512, 240), &[]);
    assert!(layout.placed[0].tag.is_some() && !layout.placed[0].ring);
    // Walled in on all eight sides by keep-outs, a name goes out on a
    // leader and its cell keeps the ring.
    // Boxed in, with a 5 px corridor east to open ground past x 144.
    let walls = [
        (60, 60, 144, 98),
        (60, 106, 144, 144),
        (60, 98, 98, 106),
        (106, 98, 144, 100),
        (106, 105, 144, 106),
    ];
    let layout = place_labels(&[n], TagAt::Centre, (0, 0, 512, 240), &walls);
    assert_eq!(
        layout.placed[0].how,
        Some(Placement::Leader { end: (148, 102) })
    );
    assert_eq!(layout.placed[0].tag, Some((148, 97, 174, 107)));
    assert!(layout.placed[0].ring);
    check_group(&[n], &layout, (0, 0, 512, 240), &walls);
    // A "?" moved off its cell gets the neutral ring.
    let layout = place_labels(&[q], TagAt::Centre, (0, 0, 512, 240), &walls);
    assert!(matches!(
        layout.placed[0].how,
        Some(Placement::Leader { .. })
    ));
    assert!(layout.placed[0].ring);
}

#[test]
fn a_tag_slides_sideways_at_the_canvas_edge() {
    // A long name above a cell at the right edge: beside and above do not
    // fit, the slid copy does.
    let keep = [(300, 94, 498, 106)];
    let n = item(500, 96, 4, LabelText::Word("Lord British's Castle"), 0, 0);
    let bounds = (0, 0, 512, 240);
    let layout = place_labels(&[n], TagAt::Centre, bounds, &keep);
    check_group(&[n], &layout, bounds, &keep);
    // Above the cell, slid left to end at the edge, its leader straight up.
    assert_eq!(layout.placed[0].tag, Some((342, 84, 512, 94)));
    assert_eq!(
        layout.placed[0].how,
        Some(Placement::Leader { end: (502, 94) })
    );
}

#[test]
fn the_unlabelled_move_to_the_front_and_the_fewest_unlabelled_win() {
    // Two names compete for the one free slot of a narrow bounds; the order
    // gives it to the nearer, and the re-run cannot do better, so exactly
    // one is unlabelled and the layout is the first run's.
    let bounds = (90, 90, 140, 106);
    let a = item(100, 96, 4, LabelText::Word("Inn"), 2, 1);
    let b = item(100, 100, 4, LabelText::Word("Inn"), 2, 4);
    let layout = place_labels(&[a, b], TagAt::Centre, bounds, &[]);
    check_group(&[a, b], &layout, bounds, &[]);
    assert!(layout.unlabelled <= 1);
    // The re-run never ends with more unlabelled than the first run.
    let mut rng = Rng(0x5EED);
    for _ in 0..50 {
        let items = random_items(&mut rng, 30, 4);
        let bounds = (0, 0, 320, 200);
        let mut order: Vec<usize> = (0..items.len()).collect();
        order.sort_by_key(|&i| (items[i].order_key(), i));
        let first = run(&items, &order, TagAt::Centre, bounds, &[]);
        let first_missing = first
            .iter()
            .zip(&items)
            .filter(|(p, item)| item.tag_size().is_some() && p.tag.is_none())
            .count();
        let layout = place_labels(&items, TagAt::Centre, bounds, &[]);
        assert!(layout.unlabelled <= first_missing);
    }
}

fn random_items(rng: &mut Rng, n: usize, size: i32) -> Vec<LabelItem> {
    (0..n)
        .map(|_| {
            let (x, y) = (rng.below(300) + 4, rng.below(180) + 4);
            let text = match rng.below(3) {
                0 => LabelText::Question,
                1 => LabelText::MarkOnly,
                _ => LabelText::Word(WORDS[rng.below(WORDS.len() as u64) as usize]),
            };
            item(x, y, size, text, rng.below(7) as u8, rng.below(5000) as u32)
        })
        .collect()
}

#[test]
fn randomised_sets_keep_every_invariant() {
    let mut rng = Rng(0x0123_4567_89AB_CDEF);
    for round in 0..150 {
        let size = [4, 6, 7, 16, 20, 24][round % 6];
        let n = 5 + rng.below(40) as usize;
        let items = random_items(&mut rng, n, size);
        let px = rng.below(280) + 10;
        let py = rng.below(160) + 10;
        let party = (px, py, px + 26, py + 26);
        let bounds = (0, 0, 320, 200);
        let at = if size >= 16 {
            TagAt::Corner
        } else {
            TagAt::Centre
        };
        let layout = place_labels(&items, at, bounds, &[party]);
        check_group(&items, &layout, bounds, &[party]);
    }
}

#[test]
fn the_order_puts_every_question_after_the_names_by_distance_only() {
    let near_q = item(0, 0, 4, LabelText::Question, 0, 1);
    let far_name = item(0, 0, 4, LabelText::Word("Sign"), 6, 900);
    assert!(far_name.order_key() < near_q.order_key());
    // A "?"'s rank and colour never reach its key (L2).
    let mut odd = near_q;
    odd.rank = 5;
    odd.colour = [1, 2, 3];
    assert_eq!(odd.order_key(), near_q.order_key());
    // Names go by category, then distance.
    let castle = item(0, 0, 4, LabelText::Word("Lord British's Castle"), 0, 900);
    let town = item(0, 0, 4, LabelText::Word("Britain"), 1, 1);
    assert!(castle.order_key() < town.order_key());
    assert_eq!(
        category_rank(FeatureCategory::Castle, false),
        0,
        "castles first"
    );
    assert!(
        category_rank(FeatureCategory::Castle, true)
            > category_rank(FeatureCategory::Shrine, false),
        "keeps after shrines"
    );
    assert!(
        category_rank(FeatureCategory::Exit, false) < category_rank(FeatureCategory::Shop, false)
    );
    assert!(
        category_rank(FeatureCategory::Shop, false)
            < category_rank(FeatureCategory::TownLink, false)
    );
    assert!(
        category_rank(FeatureCategory::TownLink, false)
            < category_rank(FeatureCategory::Sign, false)
    );
    assert!(
        category_rank(FeatureCategory::Ladder, false)
            < category_rank(FeatureCategory::Fountain, false)
    );
    assert_eq!(
        category_rank(FeatureCategory::Fountain, false),
        category_rank(FeatureCategory::Chest, false)
    );
    assert!(
        category_rank(FeatureCategory::Chest, false) < category_rank(FeatureCategory::Trap, false)
    );
}

#[test]
fn the_input_order_never_changes_the_layout() {
    let mut rng = Rng(77);
    for _ in 0..40 {
        let items = random_items(&mut rng, 25, 4);
        let layout = place_labels(&items, TagAt::Centre, (0, 0, 320, 200), &[]);
        let mut reversed = items.clone();
        reversed.reverse();
        let back = place_labels(&reversed, TagAt::Centre, (0, 0, 320, 200), &[]);
        let mut a = items.iter().zip(&layout.placed).collect::<Vec<_>>();
        let mut b = reversed.iter().zip(&back.placed).collect::<Vec<_>>();
        let key = |x: &(&LabelItem, &Placed)| (x.0.order_key(), x.0.cell, format!("{:?}", x.1));
        a.sort_by_key(key);
        b.sort_by_key(key);
        assert_eq!(a, b);
    }
}

// ---- views -----------------------------------------------------------------

fn gem_overlay(kind: ViewOverlayKind) -> Option<ViewOverlay> {
    Some(ViewOverlay {
        title: String::new(),
        text_map: String::new(),
        kind,
        mode: ViewOverlayMode::GemView,
    })
}

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

/// Undiscovered things of three kinds drawn round the party at (128, 128)
/// in open land: a nameless place marker, a live gate, a sign. `kinds`
/// says which tile goes on which of the cells.
fn three_unknowns(kinds: [u8; 3]) -> PlayState {
    let mut state = world(128, 128);
    for ((x, y), tile) in [(131, 128), (128, 126), (126, 130)].into_iter().zip(kinds) {
        set_world(&mut state, x, y, tile);
    }
    state
}

fn plan_of(state: &PlayState, know: &GemKnowledge, canvas: GemCanvas) -> (GemShown, LabelPlan) {
    let view = GemViewState::open(state, canvas, 100);
    let features = gem_features(&view.shown, know);
    let plan = gem_label_plan(&view.shown, &features, None);
    (view.shown, plan)
}

fn check_plan(plan: &LabelPlan, canvas: GemCanvas) {
    for (items, layout, bounds) in &plan.groups {
        let keep = plan.party.into_iter().collect::<Vec<_>>();
        check_group(items, layout, *bounds, &keep);
    }
    // Across groups too: no tags overlap, all inside the canvas, none on
    // the party.
    let tags = plan.tags();
    for (i, &(a, qa)) in tags.iter().enumerate() {
        let room = clearance(a, qa);
        assert!(room.0 >= 0 && room.1 >= 0);
        assert!(room.2 <= canvas.w as i32 && room.3 <= canvas.h as i32);
        if let Some(party) = plan.party {
            assert!(!overlaps(room, party));
        }
        for &(b, qb) in &tags[i + 1..] {
            assert!(gap(ink(a, qa), ink(b, qb)) >= 2, "{a:?} touches {b:?}");
        }
    }
}

#[test]
fn permuting_the_kinds_of_question_features_leaves_their_placement_unchanged() {
    let tiles = [0x10, MOONGATE_TILE, 0xA4];
    let perms = [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ];
    let know = GemKnowledge::new();
    for canvas in [C43, C169, C219] {
        let mut plans = perms.iter().map(|p| {
            let state = three_unknowns([tiles[p[0]], tiles[p[1]], tiles[p[2]]]);
            plan_of(&state, &know, canvas).1
        });
        let first = plans.next().unwrap();
        assert_eq!(first.tags().len(), 3);
        assert!(
            first
                .draws
                .iter()
                .all(|d| !matches!(d, LabelDraw::Text { .. }))
        );
        check_plan(&first, canvas);
        for plan in plans {
            assert_eq!(plan.draws, first.draws);
        }
    }
    // The same on a crowd: a dense patch of "?" of mixed kinds round the
    // party, every kind swapped round.
    let cells: Vec<(usize, usize)> = (0..6)
        .flat_map(|i| (0..3).map(move |j| (125 + i * 2, 125 + j * 3)))
        .collect();
    let kinds = [0x10, MOONGATE_TILE, 0xA4, 0x89, COLLAPSED_ENTRANCE_TILE];
    let mut reference = None;
    for shift in 0..kinds.len() {
        let mut state = world(128, 128);
        for (k, &(x, y)) in cells.iter().enumerate() {
            set_world(&mut state, x, y, kinds[(k + shift) % kinds.len()]);
        }
        let (_, plan) = plan_of(&state, &know, C169);
        check_plan(&plan, C169);
        let tags = plan.tags();
        match &reference {
            None => reference = Some(tags),
            Some(r) => assert_eq!(&tags, r),
        }
    }
}

#[test]
fn discovered_names_take_the_front_and_every_question_stays_grey() {
    // Britain's marker known, next to the "?" crowd: Britain is laid first.
    let mut state = world(84, 104);
    let britain = Scene::from_key("TOWNE:1").unwrap().byte;
    set_world(&mut state, 81, 106, 0x14);
    for (x, y) in [(82, 106), (81, 107), (80, 105), (83, 105)] {
        set_world(&mut state, x, y, 0xA4);
    }
    let mut know = GemKnowledge::new();
    know.apply(&KnowEvent::PlaceEntered { scene: britain });
    let (_, plan) = plan_of(&state, &know, C169);
    check_plan(&plan, C169);
    let (items, layout, _) = &plan.groups[0];
    let named = items
        .iter()
        .position(|item| item.text == LabelText::Word("Britain"))
        .expect("Britain");
    assert!(layout.placed[named].tag.is_some());
    for draw in &plan.draws {
        match draw {
            LabelDraw::Text { text, .. } => assert_eq!(*text, "Britain"),
            LabelDraw::Leader { colour, .. } | LabelDraw::Ring { colour, .. } => {
                assert!(*colour == NEUTRAL_GREY || *colour == FeatureCategory::Town.colour());
            }
            _ => {}
        }
    }
    for (item, p) in items.iter().zip(&layout.placed) {
        if item.unknown() && p.ring {
            assert_eq!(item.colour, NEUTRAL_GREY);
        }
    }
}

#[test]
fn the_same_input_gives_the_same_layout() {
    let state = three_unknowns([0x10, MOONGATE_TILE, 0xA4]);
    let know = GemKnowledge::new();
    for canvas in [C43, C169, C219] {
        let (shown, a) = plan_of(&state, &know, canvas);
        let (_, b) = plan_of(&state, &know, canvas);
        assert_eq!(a, b);
        let features = gem_features(&shown, &know);
        assert_eq!(gem_label_plan(&shown, &features, None), a);
    }
}

// ---- towns -----------------------------------------------------------------

fn britain() -> Scene {
    Scene::from_key("TOWNE:1").unwrap()
}

/// Britain floor 0 walled, a south gate, the party at (15, 17), with a
/// world map of grass whose every gazetteer place has its marker.
fn britain_with_land() -> (PlayState, Vec<u8>) {
    let mut grid = vec![0x44; 1024];
    for i in 0..32 {
        for (x, y) in [(i, 0), (i, 31), (0, i), (31, i)] {
            grid[y * 32 + x] = 0x4D;
        }
    }
    for x in 14..=17 {
        grid[31 * 32 + x] = 0x44;
    }
    for y in 15..=17 {
        grid[y * 32 + 31] = 0x44;
    }
    grid[5 * 32 + 5] = 0xA4;
    grid[9 * 32 + 22] = 0xC8;
    let _ = open_grid;
    let mut state = test_state(grid, 15, 17);
    state.area = Area::Town {
        scene: britain(),
        floor: 0,
    };
    state.active_view_overlay = gem_overlay(ViewOverlayKind::Surface);
    let mut world = vec![0x05; WORLD_SIDE * WORLD_SIDE];
    for entry in published_world_location_entries() {
        if entry.plane == WorldPlane::Britannia && matches!(entry.target, PlayTarget::Town(_)) {
            world[entry.y * WORLD_SIDE + entry.x] = 0x14;
        }
    }
    (state, world)
}

fn town_plan(
    state: &PlayState,
    world: &[u8],
    know: &GemKnowledge,
    canvas: GemCanvas,
) -> (TownShown, LabelPlan) {
    let view = GemViewState::open_with(
        state,
        canvas,
        100,
        TownInputs {
            know: Some(know),
            world: Some(world),
            surroundings: true,
        },
    );
    let GemShown::Town(shown) = view.shown.clone() else {
        panic!("a town view");
    };
    let features = gem_features(&view.shown, know);
    let plan = gem_label_plan(&view.shown, &features, None);
    (shown, plan)
}

fn everything_known() -> GemKnowledge {
    let mut know = GemKnowledge::new();
    for scene in 1..=32 {
        know.apply(&KnowEvent::PlaceEntered { scene });
    }
    let cells = u16::from(SIDE_WINDOW_WIDTH) * u16::from(SIDE_WINDOW_HEIGHT);
    for index in 0..cells {
        know.apply(&KnowEvent::SideCellSeen {
            scene: britain().byte,
            index,
        });
    }
    for (side, at) in [(EdgeSide::South, 15), (EdgeSide::East, 16)] {
        know.apply(&KnowEvent::TownExit(crate::gem::knowledge::ExitRun {
            scene: britain().byte,
            side,
            at,
        }));
    }
    know
}

#[test]
fn on_4_3_town_sides_keep_rings_without_names_and_the_exit_its_arrow() {
    let (state, world) = britain_with_land();
    let know = everything_known();
    let words = |plan: &LabelPlan| {
        plan.draws
            .iter()
            .filter_map(|d| match d {
                LabelDraw::Text { text, .. } => Some(*text),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    // 16:9: the places beside the town are named, and the exits.
    let (shown, wide) = town_plan(&state, &world, &know, C169);
    assert!(shown.layout.floor_px.0 >= NARROW_SIDE_PX);
    check_plan(&wide, C169);
    let wide_words = words(&wide);
    assert!(wide_words.contains(&"Exit"), "{wide_words:?}");
    let side_names = wide_words
        .iter()
        .filter(|w| !["Exit", "Sign", "Ladder", "Ladder up"].contains(w))
        .count();
    assert!(side_names > 0, "{wide_words:?}");
    // 4:3: no place name beside the town and no word under the floor, but
    // the rings and the arrows; the east exit keeps its word.
    let (shown, narrow) = town_plan(&state, &world, &know, C43);
    assert!(shown.layout.floor_px.0 < NARROW_SIDE_PX);
    check_plan(&narrow, C43);
    let floor = (
        shown.layout.floor_px.0,
        shown.layout.floor_px.0 + TOWN_FLOOR_CELLS * shown.layout.cell as i32,
    );
    let place_names: Vec<&str> = gem_features(
        &GemViewState::open_with(
            &state,
            C43,
            100,
            TownInputs {
                know: Some(&know),
                world: Some(&world),
                surroundings: true,
            },
        )
        .shown,
        &know,
    )
    .iter()
    .filter(|f| f.category().is_some_and(is_place))
    .filter_map(|f| f.label())
    .collect();
    assert!(!place_names.is_empty());
    for draw in &narrow.draws {
        if let LabelDraw::Text { text, .. } = draw {
            assert!(!place_names.contains(text), "{text} named on 4:3");
        }
    }
    let exits = narrow
        .groups
        .iter()
        .flat_map(|(items, layout, _)| items.iter().zip(&layout.placed))
        .filter(|(item, _)| item.own_mark)
        .collect::<Vec<_>>();
    assert_eq!(exits.len(), 2);
    let south = exits
        .iter()
        .max_by_key(|(item, _)| item.cell.1)
        .expect("the south exit");
    assert_eq!(south.0.text, LabelText::MarkOnly);
    let side_rings = narrow
        .draws
        .iter()
        .filter(
            |d| matches!(d, LabelDraw::Ring { cell, .. } if cell.0 < floor.0 || cell.0 >= floor.1),
        )
        .count();
    assert!(side_rings > 0);
    let arrows = narrow
        .draws
        .iter()
        .filter(|d| matches!(d, LabelDraw::Arrow { .. }))
        .count();
    assert_eq!(arrows, 2, "the south and east exits keep their arrows");
    // A "?" still shows on the narrow sides: nothing known but the sides.
    let mut seen = GemKnowledge::new();
    let cells = u16::from(SIDE_WINDOW_WIDTH) * u16::from(SIDE_WINDOW_HEIGHT);
    for index in 0..cells {
        seen.apply(&KnowEvent::SideCellSeen {
            scene: britain().byte,
            index,
        });
    }
    let (_, fresh) = town_plan(&state, &world, &seen, C43);
    check_plan(&fresh, C43);
    assert!(
        fresh
            .draws
            .iter()
            .any(|d| matches!(d, LabelDraw::Question { x, .. } if *x < floor.0 || *x >= floor.1))
    );
    assert!(words(&fresh).is_empty());
}

#[test]
fn town_labels_stay_in_their_part_and_clear_of_the_party() {
    let (state, world) = britain_with_land();
    let know = everything_known();
    for canvas in [C43, C169, C219] {
        let (shown, plan) = town_plan(&state, &world, &know, canvas);
        check_plan(&plan, canvas);
        let c = shown.layout.cell as i32;
        let (tx, ty) = shown.layout.floor_px;
        let floor = (tx, ty, tx + 32 * c, ty + 32 * c);
        // The floor group's tags stay on the floor; a side group's on its
        // side.
        let (items, layout, bounds) = &plan.groups[0];
        assert_eq!(*bounds, floor);
        for p in layout.placed.iter().filter_map(|p| p.tag) {
            assert!(p.0 >= floor.0 && p.2 <= floor.2);
        }
        assert!(!items.is_empty());
        for (_, layout, bounds) in &plan.groups[2..] {
            for tag in layout.placed.iter().filter_map(|p| p.tag) {
                assert!(tag.0 >= bounds.0 && tag.2 <= bounds.2);
            }
        }
    }
}

// ---- dungeons --------------------------------------------------------------

#[test]
fn dungeon_labels_stay_in_their_panel_clear_of_the_party_and_titles() {
    use crate::gem::dungeon::DungeonInputs;
    use crate::gem::knowledge::{DungeonMark, MarkKind};
    use crate::gem::raster::dungeon_panel_levels;
    use u5_runtime::Direction;
    use u5_runtime::enhancements::GlyphPolicy;
    use u5_runtime::test_fixtures::{dungeon_state, open_dungeon_record};
    let mut state = dungeon_state(open_dungeon_record(), 2, 1, 1);
    for lvl in 0..8u8 {
        for y in 0..8 {
            for x in 0..8 {
                let byte = if lvl == 2 { 0x00 } else { 0xB0 };
                state.grid[u5_runtime::dungeon_cell_index(lvl, x, y)] = byte;
            }
        }
    }
    for (x, y, byte) in [
        (2, 1, 0x10),
        (3, 1, 0x20),
        (4, 1, 0x50),
        (5, 1, 0x40),
        (2, 3, 0x60),
        (1, 2, 0x10),
        (2, 2, 0x20),
    ] {
        state.grid[u5_runtime::dungeon_cell_index(2, x, y)] = byte;
    }
    state.player.facing = Direction::North;
    state.active_view_overlay = gem_overlay(ViewOverlayKind::Dungeon { level: 2 });
    let record = match state.area {
        Area::Dungeon { scene, .. } => scene.record as u8,
        _ => unreachable!(),
    };
    let mut know = GemKnowledge::new();
    for (x, y, kind) in [(2, 1, MarkKind::Ladder), (4, 1, MarkKind::Fountain)] {
        know.apply(&KnowEvent::DungeonMarked(DungeonMark {
            record,
            level: 2,
            x,
            y,
            kind,
        }));
    }
    for canvas in [C43, C169, C219] {
        let view = GemViewState::open_all(
            &state,
            canvas,
            100,
            TownInputs::default(),
            DungeonInputs {
                know: Some(&know),
                glyphs: GlyphPolicy::Perceived,
            },
        );
        let GemShown::Dungeon(shown) = &view.shown else {
            panic!("a dungeon view");
        };
        let layout = DungeonLayout::new(canvas, true).unwrap();
        let panels = dungeon_panel_levels(shown, &layout, 0);
        let features = gem_features(&view.shown, &know);
        let plan = gem_label_plan(&view.shown, &features, Some((&layout, &panels)));
        let titles = dungeon_title_tags(shown, &layout, &panels)
            .into_iter()
            .map(|(t, x, y)| {
                let (w, h) = text_tag_size(&t);
                (x, y, x + w, y + h)
            })
            .collect::<Vec<_>>();
        let keep = [plan.party.into_iter().collect::<Vec<_>>(), titles].concat();
        assert!(!plan.groups.is_empty());
        for (items, layout, bounds) in &plan.groups {
            check_group(items, layout, *bounds, &keep);
        }
        check_plan(&plan, canvas);
        let texts = plan
            .draws
            .iter()
            .filter_map(|d| match d {
                LabelDraw::Text { text, .. } => Some(*text),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(
            texts.contains(&"Up") && texts.contains(&"Fountain"),
            "{texts:?}"
        );
        // Known dungeon features show no ring when labelled.
        for (items, layout, _) in &plan.groups {
            for (item, p) in items.iter().zip(&layout.placed) {
                if !item.unknown() && p.tag.is_some() {
                    assert!(!p.ring);
                }
            }
        }
    }
}

// ---- render ----------------------------------------------------------------

fn font() -> u5_runtime::FixedCellFont {
    let bytes = (0..u5_runtime::CH_FONT_LEN)
        .map(|i| {
            let (code, row) = ((i / 8) as u8, (i % 8) as u8);
            if (1..=6).contains(&row) && code != b' ' {
                0x3C
            } else {
                0
            }
        })
        .collect::<Vec<_>>();
    u5_runtime::parse_ch_font(&bytes, u5_runtime::IBM_CH_FILE).unwrap()
}

#[test]
fn the_render_draws_the_plan_and_labels_off_draws_none() {
    let mut state = world(84, 104);
    set_world(&mut state, 81, 106, 0x14);
    set_world(&mut state, 86, 104, 0xA4);
    let mut know = GemKnowledge::new();
    know.apply(&KnowEvent::PlaceEntered {
        scene: britain().byte,
    });
    let art =
        MapArt::build(&synthetic_tile_atlas(TileGraphicsDepth::Ega16)).with_fonts(font(), font());
    let view = GemViewState::open(&state, C169, 100);
    let on = GemMapToggles {
        enhanced: true,
        ..GemMapToggles::default()
    };
    let off = GemMapToggles {
        labels: false,
        ..on
    };
    let render = |t: &GemMapToggles| {
        let mut out = GemImage::default();
        render_gem_map(&state, &know, &art, t, &view, &mut out);
        out
    };
    let lit = render(&on);
    let dark = render(&off);
    assert_ne!(lit, dark);
    // Every tag of the plan has white ink (the name) or the grey ring.
    let features = gem_features(&view.shown, &know);
    let plan = gem_label_plan(&view.shown, &features, None);
    let white = [255, 255, 255, 255];
    let mut names = 0;
    for draw in &plan.draws {
        if let LabelDraw::Text { x, y, text } = draw {
            names += 1;
            let (w, h) = text_tag_size(text);
            let inked = (*x..x + w)
                .flat_map(|px| (*y..y + h).map(move |py| (px, py)))
                .filter(|&(px, py)| lit.pixel(px as u32, py as u32) == Some(white))
                .count();
            assert!(inked > 0, "{text}");
        }
    }
    assert_eq!(names, 1);
    // Labels off: the same picture as no record at all.
    let mut empty = GemImage::default();
    render_gem_map(&state, &GemKnowledge::new(), &art, &off, &view, &mut empty);
    assert_eq!(dark, empty);
}

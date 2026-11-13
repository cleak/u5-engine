//! Label placement on the gem map (Phase B design 10.2.7 "Placement" and
//! "Order", leak L2, D10.4; build plan M3-16).
//!
//! [`place_labels`] lays out one group of marks, the approved mockup's
//! `place_labels`, made exact:
//!
//! - every label is real IBM.CH text with a 1 px black outline, a tag of
//!   `8 n + 2` by 10 px ([`text_tag_size`]); an undiscovered feature's tag is
//!   the 12 x 12 "?" ([`QUESTION_TAG_PX`]);
//! - a "?" first tries its own cell (its centre, or on a dungeon cell the
//!   top-right corner, [`TagAt`]); then every tag tries the eight places
//!   beside its mark, then a leader line in eight directions at
//!   [`LEADER_RADII`] (16 to 92 px), then any of those slid sideways to stay
//!   inside the bounds, reached by a leader;
//! - a tag never overlaps another tag, another feature's mark (its cell and
//!   the two rings round it), a keep-out box (the party marker, titles) or
//!   leaves the group's bounds. A "?" keeps 1 px more round its ring, so
//!   no two labels' ink ever touch: text tags carry that pixel in their
//!   outline. A leader may not cross a tag or a mark;
//! - anything left unlabelled moves to the front and the layout runs again,
//!   up to [`LAYOUT_RUNS`] times; the run with the fewest unlabelled wins,
//!   the earliest on a tie.
//!
//! **Order** ([`LabelItem::order_key`]): discovered features first, by
//! category ([`category_rank`]) then distance from the party; every "?"
//! after them, by distance only, then by cell (L2). Nothing in a "?"
//! item depends on its kind, and two "?" on one cell are one tag, so which
//! "?" survives a crowd never hints at what it is.
//!
//! [`gem_label_plan`] builds the groups of each view as the approved mockup
//! does and returns the drawing in order ([`LabelDraw`]): outdoors one group
//! over the canvas; in a town the floor, then the north and south exits
//! round it, then each side with its own east or west exit; in a dungeon one
//! group per panel. On narrow town sides (the 4:3 canvas, D10.4) a
//! discovered place keeps its ring without its name and the north and
//! south exits their arrows without `Exit`; a "?" still shows.
//!
//! Everything here is pure: the same features and layout give the same
//! plan.

use std::collections::BTreeSet;
use std::sync::OnceLock;

use u5_runtime::{PlayTarget, WorldEntryNarrationClass, published_world_location_entries};

use super::dungeon::{DungeonShown, DungeonSlot};
use super::features::{FeatureCategory, FeatureCell, GemFeature, NEUTRAL_GREY, QUESTION_TAG_PX};
use super::knowledge::{EdgeSide, FeatureKey};
use super::layout::{DUNGEON_PANEL_CELLS, DungeonLayout, GemCanvas, TOWN_FLOOR_CELLS};
use super::shown::{GemShown, OutdoorShown, TownShown};

/// A box on the canvas, `(x0, y0, x1, y1)`, the ends exclusive.
pub type Rect = (i32, i32, i32, i32);

/// The leader radii in pixels, from the cell's centre (10.2.7: 16 to 92).
pub const LEADER_RADII: [i32; 7] = [16, 24, 34, 46, 60, 76, 92];

/// The leader directions in degrees, in the order they are tried: east,
/// west, then the diagonals below, the diagonals above, north and south
/// (screen y grows down; 90 is up).
pub const LEADER_ANGLES: [i32; 8] = [0, 180, 315, 225, 45, 135, 270, 90];

/// How many times the layout runs at most (10.2.7).
pub const LAYOUT_RUNS: usize = 4;

/// The extra clearance round a "?" tag, so its ring never touches another
/// label's ink (text tags carry this pixel as their outline).
pub const QUESTION_CLEARANCE: i32 = 1;

/// A town side narrower than this many pixels shows rings without names
/// (D10.4): 64 px at 4:3, 144 px at 16:9.
pub const NARROW_SIDE_PX: i32 = 96;

/// Where a "?" first tries to sit on its own cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TagAt {
    /// Centred on the cell (outdoors and towns).
    Centre,
    /// On the cell's top-right corner, 3 px out, so a dungeon glyph stays
    /// readable.
    Corner,
}

/// What a label shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LabelText {
    /// A discovered feature's word.
    Word(&'static str),
    /// A discovered feature shown by its mark alone (D10.4): no tag.
    MarkOnly,
    /// An undiscovered feature: the "?" tag.
    Question,
}

/// One mark to label.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LabelItem {
    /// The feature's cell on the canvas (an exit: its arrow's box).
    pub cell: Rect,
    pub text: LabelText,
    /// The category order of a discovered feature ([`category_rank`]);
    /// ignored for a "?".
    pub rank: u8,
    /// The squared distance from the party, in cells.
    pub distance: u32,
    /// The ring colour: the category's, or [`NEUTRAL_GREY`] for a "?".
    pub colour: [u8; 3],
    /// The feature draws its own mark (a town exit's arrow) instead of a
    /// ring.
    pub own_mark: bool,
    /// Whether a discovered feature's ring is drawn when its tag is placed
    /// (outdoors and towns; in a dungeon the label alone marks it).
    pub ring_known: bool,
}

impl LabelItem {
    /// Whether it is a "?".
    pub fn unknown(&self) -> bool {
        self.text == LabelText::Question
    }

    /// The tag's size, or `None` for a mark without a tag.
    pub fn tag_size(&self) -> Option<(i32, i32)> {
        match self.text {
            LabelText::Word(word) => Some(text_tag_size(word)),
            LabelText::MarkOnly => None,
            LabelText::Question => Some((QUESTION_TAG_PX, QUESTION_TAG_PX)),
        }
    }

    /// The mark's box: the cell and the two 1 px rings round it.
    pub fn mark_box(&self) -> Rect {
        grow(self.cell, 2)
    }

    /// The order key (10.2.7, L2): discovered first, by category, distance,
    /// label length and cell; then every "?" by distance and cell only.
    pub fn order_key(&self) -> (u8, u8, u32, usize, i32, i32) {
        match self.text {
            LabelText::Question => (1, 0, self.distance, 0, self.cell.1, self.cell.0),
            LabelText::Word(word) => (
                0,
                self.rank,
                self.distance,
                word.chars().count(),
                self.cell.1,
                self.cell.0,
            ),
            LabelText::MarkOnly => (0, self.rank, self.distance, 0, self.cell.1, self.cell.0),
        }
    }
}

/// How a tag was placed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Placement {
    /// A "?" on its own cell.
    OnCell,
    /// Beside its mark.
    Beside,
    /// Out on a leader from the cell's centre to `end`.
    Leader { end: (i32, i32) },
}

/// One item's outcome.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placed {
    /// The tag's box, or `None` when it found no place or has no tag.
    pub tag: Option<Rect>,
    pub how: Option<Placement>,
    /// Whether the cell gets its ring.
    pub ring: bool,
}

/// One group's layout, in the input's order.
#[derive(Clone, Debug, PartialEq)]
pub struct GroupLayout {
    pub placed: Vec<Placed>,
    /// How many items with a tag found no place.
    pub unlabelled: usize,
}

/// A text tag's size: 8 px a character and the 1 px outline round it.
pub fn text_tag_size(text: &str) -> (i32, i32) {
    (8 * text.chars().count() as i32 + 2, 10)
}

fn grow(r: Rect, k: i32) -> Rect {
    (r.0 - k, r.1 - k, r.2 + k, r.3 + k)
}

/// Whether two boxes share a pixel.
pub fn overlaps(a: Rect, b: Rect) -> bool {
    a.0 < b.2 && b.0 < a.2 && a.1 < b.3 && b.1 < a.3
}

fn centre(r: Rect) -> (f64, f64) {
    (f64::from(r.0 + r.2) / 2.0, f64::from(r.1 + r.3) / 2.0)
}

/// The box a tag at `b` keeps clear: a "?" its 1 px clearance more.
fn clearance(b: Rect, unknown: bool) -> Rect {
    if unknown {
        grow(b, QUESTION_CLEARANCE)
    } else {
        b
    }
}

/// The candidates of one item, in the order they are tried.
fn candidates(item: &LabelItem, at: TagAt, bounds: Rect) -> Vec<(i32, i32, Option<Placement>)> {
    let Some((w, h)) = item.tag_size() else {
        return Vec::new();
    };
    let cell = item.cell;
    let mut out = Vec::new();
    if item.unknown() {
        let (x, y) = match at {
            TagAt::Centre => {
                let (cx, cy) = (
                    (cell.0 + cell.2).div_euclid(2),
                    (cell.1 + cell.3).div_euclid(2),
                );
                (cx - w / 2, cy - h / 2)
            }
            TagAt::Corner => (cell.2 - w + 3, cell.1 - 3),
        };
        out.push((x, y, Some(Placement::OnCell)));
        // Cut by the bounds (a cell at the edge, a town exit's arrow in the
        // strip below the floor): the same tag pushed just inside, while it
        // still covers its cell.
        let k = QUESTION_CLEARANCE;
        let (sx, sy) = (
            x.max(bounds.0 + k).min(bounds.2 - k - w),
            y.max(bounds.1 + k).min(bounds.3 - k - h),
        );
        if (sx, sy) != (x, y) && overlaps((sx, sy, sx + w, sy + h), cell) {
            out.push((sx, sy, Some(Placement::OnCell)));
        }
    }
    // Beside the mark: right, left, above, below, then the four corners.
    let (x0, y0, x1, y1) = item.mark_box();
    let cy = (y0 + y1).div_euclid(2) - h / 2;
    let cx = (x0 + x1).div_euclid(2) - w / 2;
    for (x, y) in [
        (x1, cy),
        (x0 - w, cy),
        (cx, y0 - h),
        (cx, y1),
        (x1, y0 - h + 2),
        (x0 - w, y0 - h + 2),
        (x1, y1 - 2),
        (x0 - w, y1 - 2),
    ] {
        out.push((x, y, Some(Placement::Beside)));
    }
    // Leaders: the tag's near edge at the leader's end.
    let (fx, fy) = centre(cell);
    for r in LEADER_RADII {
        for angle in LEADER_ANGLES {
            let rad = f64::from(angle).to_radians();
            let (dx, dy) = (rad.cos(), -rad.sin());
            let tx = fx + dx * f64::from(r);
            let ty = fy + dy * f64::from(r);
            let ox = if dx > 0.3 {
                0.0
            } else if dx < -0.3 {
                f64::from(w)
            } else {
                f64::from(w) / 2.0
            };
            let oy = if dy > 0.3 {
                0.0
            } else if dy < -0.3 {
                f64::from(h)
            } else {
                f64::from(h) / 2.0
            };
            out.push((
                (tx - ox).round_ties_even() as i32,
                (ty - oy).round_ties_even() as i32,
                Some(Placement::Leader {
                    end: (tx as i32, ty as i32),
                }),
            ));
        }
    }
    // The same places slid sideways into the bounds, reached by a leader.
    let clamp = |x: i32| x.max(bounds.0).min(bounds.2 - w);
    let (ccx, ccy) = (
        (cell.0 + cell.2).div_euclid(2),
        (cell.1 + cell.3).div_euclid(2),
    );
    let slid = out
        .iter()
        .filter(|(x, ..)| clamp(*x) != *x)
        .map(|&(x, y, _)| {
            let end_y = y + if y < ccy { h } else { 0 };
            (clamp(x), y, Some(Placement::Leader { end: (ccx, end_y) }))
        })
        .collect::<Vec<_>>();
    out.extend(slid);
    out
}

/// The pixels of a Bresenham line from `a` to `b`, both ends included.
pub fn line_points(a: (i32, i32), b: (i32, i32)) -> Vec<(i32, i32)> {
    let (mut x, mut y) = a;
    let (dx, dy) = ((b.0 - x).abs(), -(b.1 - y).abs());
    let (sx, sy) = (if x < b.0 { 1 } else { -1 }, if y < b.1 { 1 } else { -1 });
    let mut err = dx + dy;
    let mut out = Vec::new();
    loop {
        out.push((x, y));
        if (x, y) == b {
            return out;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}

/// The first pixel of a leader from a cell's centre.
pub fn leader_start(from: (f64, f64)) -> (i32, i32) {
    (from.0 as i32, from.1 as i32)
}

/// The boxes a leader from `cell`'s centre to `end` covers, its 3 px
/// black line included, outside `own` (its mark) and `tag`.
fn leader_footprint(cell: Rect, end: (i32, i32), own: Rect, tag: Rect) -> Vec<Rect> {
    line_points(leader_start(centre(cell)), end)
        .into_iter()
        .map(|(x, y)| (x - 1, y - 1, x + 2, y + 2))
        .filter(|&b| !overlaps(shrink1(b), own) && !overlaps(shrink1(b), tag))
        .collect()
}

/// A footprint box's own pixel.
fn shrink1(b: Rect) -> Rect {
    (b.0 + 1, b.1 + 1, b.2 - 1, b.3 - 1)
}

/// One run of the layout over `order` (indices into `items`).
fn run(
    items: &[LabelItem],
    order: &[usize],
    at: TagAt,
    bounds: Rect,
    keepout: &[Rect],
) -> Vec<Placed> {
    let marks: Vec<Rect> = items.iter().map(LabelItem::mark_box).collect();
    let mut placed: Vec<Rect> = keepout.to_vec();
    // The leaders already drawn: no later tag may cover one.
    let mut lines: Vec<Rect> = Vec::new();
    let mut out = vec![
        Placed {
            tag: None,
            how: None,
            ring: false,
        };
        items.len()
    ];
    for &i in order {
        let item = &items[i];
        let unknown = item.unknown();
        let free = |b: Rect, placed: &[Rect]| {
            let c = clearance(b, unknown);
            if c.0 < bounds.0 || c.1 < bounds.1 || c.2 > bounds.2 || c.3 > bounds.3 {
                return false;
            }
            if placed.iter().any(|&q| overlaps(c, q)) || lines.iter().any(|&q| overlaps(c, q)) {
                return false;
            }
            !marks
                .iter()
                .enumerate()
                .any(|(k, &m)| k != i && overlaps(c, m))
        };
        // A feature whose own cell lies under a keep-out box (a place next
        // to the party, under its 16 px marker at 4 px a cell) may lead out
        // across that box: otherwise every leader from it is blocked and a
        // discovered name next to the party is never shown. Its tag still
        // keeps clear of the box, and the box is drawn over the leader.
        let under: Vec<bool> = keepout.iter().map(|&k| overlaps(item.cell, k)).collect();
        let clear_line =
            |end: (i32, i32), tag: Rect, placed: &[Rect]| {
                leader_footprint(item.cell, end, marks[i], tag)
                    .into_iter()
                    .all(|b| {
                        !placed.iter().enumerate().any(|(n, &q)| {
                            overlaps(b, q) && !under.get(n).copied().unwrap_or(false)
                        }) && !marks
                            .iter()
                            .enumerate()
                            .any(|(k, &m)| k != i && overlaps(b, m))
                    })
            };
        let Some((w, h)) = item.tag_size() else {
            // A mark without a name (D10.4): its ring only.
            out[i] = Placed {
                tag: None,
                how: None,
                ring: !item.own_mark,
            };
            continue;
        };
        let mut done = None;
        for (x, y, how) in candidates(item, at, bounds) {
            let b = (x, y, x + w, y + h);
            if let Some(Placement::Leader { end }) = how
                && !clear_line(end, b, &placed)
            {
                continue;
            }
            if how != Some(Placement::OnCell) && overlaps(b, marks[i]) {
                continue;
            }
            if free(b, &placed) {
                done = Some((b, how));
                break;
            }
        }
        out[i] = match done {
            Some((b, how)) => {
                placed.push(clearance(b, unknown));
                if let Some(Placement::Leader { end }) = how {
                    lines.extend(leader_footprint(item.cell, end, marks[i], b));
                }
                Placed {
                    tag: Some(b),
                    how,
                    ring: how != Some(Placement::OnCell)
                        && !item.own_mark
                        && (item.ring_known || unknown),
                }
            }
            None => Placed {
                tag: None,
                how: None,
                ring: !item.own_mark,
            },
        };
    }
    out
}

/// The layout of one group (10.2.7): `items` inside `bounds`, clear of
/// `keepout`, with a "?" first tried `at` its cell. The result is in the
/// input's order; the input order itself does not matter.
pub fn place_labels(items: &[LabelItem], at: TagAt, bounds: Rect, keepout: &[Rect]) -> GroupLayout {
    let mut order: Vec<usize> = (0..items.len()).collect();
    order.sort_by_key(|&i| (items[i].order_key(), i));
    let missing = |placed: &[Placed]| {
        placed
            .iter()
            .zip(items)
            .filter(|(p, item)| item.tag_size().is_some() && p.tag.is_none())
            .count()
    };
    let mut best: Option<(usize, Vec<Placed>)> = None;
    for _ in 0..LAYOUT_RUNS {
        let placed = run(items, &order, at, bounds, keepout);
        let miss = missing(&placed);
        let failed: BTreeSet<usize> = (0..items.len())
            .filter(|&i| items[i].tag_size().is_some() && placed[i].tag.is_none())
            .collect();
        if best.as_ref().is_none_or(|(m, _)| miss < *m) {
            best = Some((miss, placed));
        }
        if miss == 0 {
            break;
        }
        let (front, back): (Vec<usize>, Vec<usize>) =
            order.iter().partition(|i| failed.contains(i));
        let next = [front, back].concat();
        if next == order {
            break;
        }
        order = next;
    }
    let (unlabelled, placed) = best.unwrap_or_default();
    GroupLayout { placed, unlabelled }
}

/// The placement order of a discovered feature's category (10.2.7
/// "Order"): outdoors castles, towns, shrines, keeps and dungeons,
/// dwellings, moongates, signs; in towns exits, shops, ladders and stairs,
/// signs; in dungeons ladders, fountains and chests, traps.
pub fn category_rank(category: FeatureCategory, keep: bool) -> u8 {
    match category {
        FeatureCategory::Castle if keep => 3,
        FeatureCategory::Castle => 0,
        FeatureCategory::Town | FeatureCategory::Exit | FeatureCategory::Ladder => 1,
        FeatureCategory::Shrine
        | FeatureCategory::OtherPlace
        | FeatureCategory::Shop
        | FeatureCategory::Fountain
        | FeatureCategory::Chest => 2,
        FeatureCategory::Dungeon | FeatureCategory::TownLink | FeatureCategory::Trap => 3,
        FeatureCategory::Dwelling => 4,
        FeatureCategory::Moongate => 5,
        FeatureCategory::Sign => 6,
    }
}

/// Whether a place key is a keep (gazetteer §5.1 narration class), which
/// orders after the towns and shrines.
pub fn is_keep(key: Option<FeatureKey>) -> bool {
    static KEEPS: OnceLock<Vec<u8>> = OnceLock::new();
    let keeps = KEEPS.get_or_init(|| {
        published_world_location_entries()
            .into_iter()
            .filter(|entry| entry.narration_class == Some(WorldEntryNarrationClass::Keep))
            .filter_map(|entry| match entry.target {
                PlayTarget::Town(scene) => Some(scene.byte),
                _ => None,
            })
            .collect()
    });
    matches!(key, Some(FeatureKey::Place { scene }) if keeps.contains(&scene))
}

/// One step of the labels' drawing, at full brightness.
#[derive(Clone, Debug, PartialEq)]
pub enum LabelDraw {
    /// A town exit's IBM.CH arrow, outlined, top-left at `(x, y)`.
    Arrow { code: u8, x: i32, y: i32 },
    /// A 1 px leader in `colour` over a 3 px black line.
    Leader {
        from: (f64, f64),
        to: (i32, i32),
        colour: [u8; 3],
    },
    /// A 1 px ring in `colour` just outside `cell`, and a black one
    /// outside that.
    Ring { cell: Rect, colour: [u8; 3] },
    /// A text tag, top-left at `(x, y)`.
    Text { text: &'static str, x: i32, y: i32 },
    /// A "?" tag, top-left at `(x, y)`.
    Question { x: i32, y: i32 },
}

/// The labels of one open view: what to draw, in order, and each group's
/// items and layout for the checks.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LabelPlan {
    pub draws: Vec<LabelDraw>,
    pub groups: Vec<(Vec<LabelItem>, GroupLayout, Rect)>,
    /// The keep-out boxes every group respects: the party marker's.
    pub party: Option<Rect>,
}

impl LabelPlan {
    /// Every placed tag, with whether it is a "?".
    pub fn tags(&self) -> Vec<(Rect, bool)> {
        self.groups
            .iter()
            .flat_map(|(items, layout, _)| {
                items
                    .iter()
                    .zip(&layout.placed)
                    .filter_map(|(item, p)| Some((p.tag?, item.unknown())))
            })
            .collect()
    }

    /// Every item's mark box.
    pub fn marks(&self) -> Vec<Rect> {
        self.groups
            .iter()
            .flat_map(|(items, ..)| items.iter().map(LabelItem::mark_box))
            .collect()
    }

    /// How many tags found no place.
    pub fn unlabelled(&self) -> usize {
        self.groups
            .iter()
            .map(|(_, layout, _)| layout.unlabelled)
            .sum()
    }

    fn add(
        &mut self,
        items: Vec<LabelItem>,
        arrows: &[(u8, Rect)],
        at: TagAt,
        bounds: Rect,
        keepout: &[Rect],
    ) {
        let layout = place_labels(&items, at, bounds, keepout);
        for &(code, cell) in arrows {
            self.draws.push(LabelDraw::Arrow {
                code,
                x: cell.0,
                y: cell.1,
            });
        }
        for (item, p) in items.iter().zip(&layout.placed) {
            if let Some(Placement::Leader { end }) = p.how {
                self.draws.push(LabelDraw::Leader {
                    from: centre(item.cell),
                    to: end,
                    colour: item.colour,
                });
            }
        }
        for (item, p) in items.iter().zip(&layout.placed) {
            if p.ring {
                self.draws.push(LabelDraw::Ring {
                    cell: item.cell,
                    colour: item.colour,
                });
            }
        }
        for (item, p) in items.iter().zip(&layout.placed) {
            let Some(tag) = p.tag else { continue };
            self.draws.push(match item.text {
                LabelText::Word(text) => LabelDraw::Text {
                    text,
                    x: tag.0,
                    y: tag.1,
                },
                _ => LabelDraw::Question { x: tag.0, y: tag.1 },
            });
        }
        self.groups.push((items, layout, bounds));
    }
}

/// The party marker's keep-out box (10.2.7): the marker of
/// `raster::draw_party_marker` and 1 px more, for the party cell whose
/// top-left is `cell_px`.
pub fn party_keepout(cell_px: (i32, i32), cell: u32) -> Rect {
    let s = (cell.max(super::raster::PARTY_SPRITE_PX)) as i32;
    let c = cell as i32;
    let x = cell_px.0 + c / 2 - s / 2;
    let y = cell_px.1 + c / 2 - s / 2;
    (x - 5, y - 5, x + s + 5, y + s + 5)
}

/// The label item of a feature on `cell` (`x, y, w, h`).
fn item_of(
    feature: &GemFeature,
    cell: (i32, i32, u32, u32),
    ring_known: bool,
    names: bool,
) -> LabelItem {
    let rect = (
        cell.0,
        cell.1,
        cell.0 + cell.2 as i32,
        cell.1 + cell.3 as i32,
    );
    let text = match feature.label() {
        None => LabelText::Question,
        Some(_) if !names => LabelText::MarkOnly,
        Some(word) => LabelText::Word(word),
    };
    LabelItem {
        cell: rect,
        text,
        rank: feature
            .category()
            .map_or(0, |c| category_rank(c, is_keep(feature.key()))),
        distance: feature.distance,
        colour: feature.ring_colour(),
        own_mark: false,
        ring_known,
    }
}

/// Whether a cell lies wholly on the canvas: a cut edge cell carries no
/// mark (the approved mockups' rule).
fn on_canvas(canvas: GemCanvas, (x, y, w, h): (i32, i32, u32, u32)) -> bool {
    x >= 0 && y >= 0 && x + w as i32 <= canvas.w as i32 && y + h as i32 <= canvas.h as i32
}

/// Keeps one "?" per cell (the first in feature order), so two
/// undiscovered features on one cell are one "?" whatever their kinds.
fn dedupe_questions(items: &mut Vec<LabelItem>) {
    let mut seen = BTreeSet::new();
    items.retain(|item| !item.unknown() || seen.insert(item.cell));
}

/// The outdoor plan: one group over the canvas, clear of the party.
pub fn outdoor_label_plan(shown: &OutdoorShown, features: &[GemFeature]) -> LabelPlan {
    let canvas = shown.layout.canvas;
    let mut items = features
        .iter()
        .filter_map(|feature| {
            let FeatureCell::Outdoor { dx, dy } = feature.cell else {
                return None;
            };
            let cell = shown.layout.cell_rect(dx, dy);
            on_canvas(canvas, cell).then(|| item_of(feature, cell, true, true))
        })
        .collect::<Vec<_>>();
    dedupe_questions(&mut items);
    let party = party_keepout(shown.layout.party_px, shown.layout.cell);
    let mut plan = LabelPlan {
        party: Some(party),
        ..LabelPlan::default()
    };
    let bounds = (0, 0, canvas.w as i32, canvas.h as i32);
    plan.add(items, &[], TagAt::Centre, bounds, &[party]);
    plan
}

/// The arrow box of a town exit: a one-character text tag centred one cell
/// outside the floor's edge.
fn exit_arrow_box(shown: &TownShown, fx: i32, fy: i32, side: EdgeSide) -> Rect {
    let (dx, dy) = match side {
        EdgeSide::North => (0, -1),
        EdgeSide::South => (0, 1),
        EdgeSide::West => (-1, 0),
        EdgeSide::East => (1, 0),
    };
    let (x0, y0, w, h) = shown.layout.cell_rect(fx + dx, fy + dy);
    let (cx, cy) = (x0 + w as i32 / 2, y0 + h as i32 / 2);
    (cx - 5, cy - 5, cx + 5, cy + 5)
}

/// Whether a category is a place of the gazetteer (a town, castle, keep,
/// dwelling, dungeon entrance, shrine or the Codex).
pub fn is_place(category: FeatureCategory) -> bool {
    matches!(
        category,
        FeatureCategory::Town
            | FeatureCategory::Castle
            | FeatureCategory::Dwelling
            | FeatureCategory::Dungeon
            | FeatureCategory::Shrine
            | FeatureCategory::OtherPlace
    )
}

/// The town plan (10.2.5, 10.2.7), the approved mockup's passes: the
/// floor's features inside the floor; the north and south exits round it;
/// then each side's places with that side's exit, inside the side. On a
/// side narrower than [`NARROW_SIDE_PX`] (4:3, D10.4), a discovered place
/// keeps its ring without its name and the north and south exits their
/// arrows without `Exit`, as in the approved 4:3 mockup; signs, moongates
/// and the east and west exits keep their short words where they fit.
pub fn town_label_plan(shown: &TownShown, features: &[GemFeature]) -> LabelPlan {
    let layout = shown.layout;
    let canvas = layout.canvas;
    let (w, h) = (canvas.w as i32, canvas.h as i32);
    let c = layout.cell as i32;
    let (tx, ty) = layout.floor_px;
    let floor = (tx, ty, tx + TOWN_FLOOR_CELLS * c, ty + TOWN_FLOOR_CELLS * c);
    let names = tx >= NARROW_SIDE_PX;
    let (px, py) = (i32::from(shown.party.0), i32::from(shown.party.1));
    let (cx, cy, _, _) = layout.cell_rect(px, py);
    let party = party_keepout((cx, cy), layout.cell);
    let mut inner = Vec::new();
    let mut exits: Vec<(EdgeSide, LabelItem, u8)> = Vec::new();
    let mut flank_w = Vec::new();
    let mut flank_e = Vec::new();
    for feature in features {
        let FeatureCell::Town { fx, fy, exit } = feature.cell else {
            continue;
        };
        if let Some(side) = exit {
            let arrow = exit_arrow_box(shown, fx, fy, side);
            // D10.4: under and over the floor the strip is too thin for a
            // word on 4:3; an east or west exit keeps it when it fits.
            let word = names || matches!(side, EdgeSide::East | EdgeSide::West);
            let mut item = item_of(feature, (arrow.0, arrow.1, 10, 10), true, word);
            item.own_mark = true;
            exits.push((side, item, super::raster::exit_arrow(side)));
            continue;
        }
        let cell = layout.cell_rect(fx, fy);
        if !on_canvas(canvas, cell) {
            continue;
        }
        if (0..TOWN_FLOOR_CELLS).contains(&fx) && (0..TOWN_FLOOR_CELLS).contains(&fy) {
            inner.push(item_of(feature, cell, true, true));
        } else {
            // D10.4: a place on a narrow side keeps its ring without its
            // name; a sign or a moongate keeps its short word.
            let word = names || !feature.category().is_some_and(is_place);
            let item = item_of(feature, cell, true, word);
            if fx < 0 {
                flank_w.push(item);
            } else if fx >= TOWN_FLOOR_CELLS {
                flank_e.push(item);
            }
        }
    }
    for items in [&mut inner, &mut flank_w, &mut flank_e] {
        dedupe_questions(items);
    }
    let mut plan = LabelPlan {
        party: Some(party),
        ..LabelPlan::default()
    };
    let exit_cells: Vec<Rect> = exits.iter().map(|(_, item, _)| item.cell).collect();
    let inner_cells: Vec<Rect> = inner.iter().map(|item| item.cell).collect();
    let flank_cells: Vec<Rect> = flank_w
        .iter()
        .chain(&flank_e)
        .map(|item| item.cell)
        .collect();
    // The floor.
    let keep = [vec![party], exit_cells.clone()].concat();
    plan.add(inner, &[], TagAt::Centre, floor, &keep);
    // The north and south exits, round the floor.
    let ns = exits
        .iter()
        .filter(|(side, ..)| matches!(side, EdgeSide::North | EdgeSide::South))
        .collect::<Vec<_>>();
    let core = (floor.0 + 6, floor.1 + 6, floor.2 - 6, floor.3 - 6);
    let keep = [
        vec![party],
        plan.tags()
            .into_iter()
            .map(|(r, q)| clearance(r, q))
            .collect(),
        inner_cells.clone(),
        flank_cells,
        vec![core],
    ]
    .concat();
    plan.add(
        ns.iter().map(|(_, item, _)| *item).collect(),
        &ns.iter()
            .map(|(_, item, code)| (*code, item.cell))
            .collect::<Vec<_>>(),
        TagAt::Centre,
        (0, 0, w, h),
        &keep,
    );
    // Each side with its own exit, inside the side.
    for (side, flank, bounds) in [
        (EdgeSide::West, flank_w, (0, 0, tx, h)),
        (EdgeSide::East, flank_e, (floor.2, 0, w, h)),
    ] {
        let keep = [
            vec![party],
            plan.tags()
                .into_iter()
                .map(|(r, q)| clearance(r, q))
                .collect(),
            inner_cells.clone(),
            exit_cells.clone(),
        ]
        .concat();
        let mine = exits
            .iter()
            .filter(|(s, ..)| *s == side)
            .collect::<Vec<_>>();
        let mut items = flank;
        items.extend(mine.iter().map(|(_, item, _)| *item));
        let arrows = mine
            .iter()
            .map(|(_, item, code)| (*code, item.cell))
            .collect::<Vec<_>>();
        plan.add(items, &arrows, TagAt::Centre, bounds, &keep);
    }
    plan
}

/// The dungeon titles' tags (10.2.6) on the panels of `panels`: each
/// `(text, x, y)`, centred over its panel or, with no room above (4:3), in
/// the margin to the left, on two lines when one is too wide.
pub fn dungeon_title_tags(
    shown: &DungeonShown,
    layout: &DungeonLayout,
    panels: &[(u8, i32)],
) -> Vec<(String, i32, i32)> {
    let p = layout.panel_px() as i32;
    let mut out = Vec::new();
    for &(index, level) in panels {
        let Some(title) = shown.title(level) else {
            continue;
        };
        let (px, py) = layout.panel_origin(index);
        let (w, h) = text_tag_size(&title);
        let ty = py - h - 3;
        if ty >= 0 {
            out.push((title, px + (p - w) / 2, ty));
            continue;
        }
        let lines = if w <= px {
            vec![title.clone()]
        } else {
            let words = title.split(' ').collect::<Vec<_>>();
            let cut = words.len().saturating_sub(2);
            vec![words[..cut].join(" "), words[cut..].join(" ")]
        };
        for (k, line) in lines
            .into_iter()
            .filter(|line| !line.is_empty())
            .enumerate()
        {
            let (w, _) = text_tag_size(&line);
            out.push((line, ((px - w) / 2).max(0), 6 + 10 * k as i32));
        }
    }
    out
}

/// The dungeon plan (10.2.6, 10.2.7): one group per panel with cells, its
/// features inside the panel's column and 6 px beside it, a "?" first on
/// its cell's top-right corner, clear of the party's cell and the titles.
/// A discovered feature's label marks it; its ring shows only when the
/// label finds no place.
pub fn dungeon_label_plan(
    shown: &DungeonShown,
    layout: &DungeonLayout,
    panels: &[(u8, i32)],
    features: &[GemFeature],
) -> LabelPlan {
    let c = layout.cell as i32;
    let p = layout.panel_px() as i32;
    let h = layout.canvas.h as i32;
    let mut keep: Vec<Rect> = Vec::new();
    let mut party = None;
    if let Some(&(index, _)) = panels
        .iter()
        .find(|(_, level)| *level == i32::from(shown.level))
    {
        let (ox, oy) = layout.panel_origin(index);
        let (x, y) = (
            ox + i32::from(shown.party.0) * c,
            oy + i32::from(shown.party.1) * c,
        );
        let r = (x - 1, y - 1, x + c + 1, y + c + 1);
        keep.push(r);
        party = Some(r);
    }
    for (text, x, y) in dungeon_title_tags(shown, layout, panels) {
        let (w, th) = text_tag_size(&text);
        keep.push((x, y, x + w, y + th));
    }
    let mut plan = LabelPlan {
        party,
        ..LabelPlan::default()
    };
    for &(index, level) in panels {
        if !matches!(shown.slot(level), DungeonSlot::Level { cells: true, .. }) {
            continue;
        }
        let (ox, oy) = layout.panel_origin(index);
        let mut items = features
            .iter()
            .filter_map(|feature| {
                let FeatureCell::Dungeon { level: l, x, y } = feature.cell else {
                    return None;
                };
                (i32::from(l) == level && u32::from(x) < DUNGEON_PANEL_CELLS).then(|| {
                    item_of(
                        feature,
                        (
                            ox + i32::from(x) * c,
                            oy + i32::from(y) * c,
                            layout.cell,
                            layout.cell,
                        ),
                        false,
                        true,
                    )
                })
            })
            .collect::<Vec<_>>();
        dedupe_questions(&mut items);
        plan.add(items, &[], TagAt::Corner, (ox - 6, 0, ox + p + 6, h), &keep);
    }
    plan
}

/// The label plan of an open view; empty for an empty view.
pub fn gem_label_plan(
    shown: &GemShown,
    features: &[GemFeature],
    dungeon: Option<(&DungeonLayout, &[(u8, i32)])>,
) -> LabelPlan {
    match shown {
        GemShown::Empty => LabelPlan::default(),
        GemShown::Outdoor(shown) => outdoor_label_plan(shown, features),
        GemShown::Town(shown) => town_label_plan(shown, features),
        GemShown::Dungeon(shown) => dungeon.map_or_else(LabelPlan::default, |(layout, panels)| {
            dungeon_label_plan(shown, layout, panels, features)
        }),
    }
}

/// The colour a "?" leader and ring take: the neutral grey (L1).
pub const QUESTION_COLOUR: [u8; 3] = NEUTRAL_GREY;

#[cfg(test)]
#[path = "labels_tests.rs"]
mod tests;

//! The no-spoiler audit of one open gem map (Phase B design 1.4.7, 10.2.3,
//! 10.2.7, 10.2.8, 10.2.9 L1-L9 and the 10.6 no-spoiler suite; build plan
//! M3-18).
//!
//! [`audit_gem_view`] takes a view exactly as the window draws it (the
//! game state, the record, the toggles and the open [`GemViewState`]) and
//! returns every way the map would tell the player more than the game has
//! shown them. The tests of this crate run it over synthetic and real
//! corpora (`spoiler_tests`); the Bevy shell's spoiler test runs it on the
//! view its own window holds. It reads only, and is never called while a
//! game runs.
//!
//! The checks, in the order of 10.6:
//!
//! - every "?" lies on a shown cell whose drawn tile is one of its kind's
//!   (10.2.7), its ring is [`NEUTRAL_GREY`], and it has no category, colour
//!   or label;
//! - every word drawn belongs to a discovered feature of the view, or is a
//!   dungeon title; a discovered feature's ring is its category's;
//! - every "?" tag follows every known label in its group's order, by
//!   distance only (L2);
//! - `Underworld` appears only once the record holds the visit;
//! - under `perceived`, a hidden dungeon byte (`0xD?`, `0x61`, `0x69`,
//!   `0x62`) is drawn as its perceived class unless the record holds it
//!   found;
//! - no cell of another level is drawn unless remembered, and no flood
//!   cell lies off the party's level; a level panel appears only with
//!   evidence, `Underworld` named only after a visit, and a link box only
//!   with both ends shown;
//! - no moongate mark while its cell does not hold `0xDC`, unless that gate
//!   is in the record;
//! - a town view's side cells are black unless the record holds them seen,
//!   both in the shown set and in the picture.
//!
//! [`flip_changes`] covers the last item of the list: flipping one
//! feature's record bit changes only that feature's mark and label.
//! The words themselves (`texts`) are returned so a caller can compare
//! them with the game's own sign and conversation text, which this crate
//! never reads.

use u5_runtime::PlayState;
use u5_runtime::enhancements::{GemMapToggles, GlyphPolicy};
use u5_runtime::predicates::{foot_terrain_accepts, is_town_door_tile};

use super::dungeon::{DungeonLit, DungeonShown, DungeonSlot, leads};
use super::features::{
    FeatureCategory, FeatureCell, GemFeature, MOONGATE_TILE, NEUTRAL_GREY, SHOP_SIGN_TILES,
    SIGN_TILES, gem_features, is_place_marker,
};
use super::knowledge::{FeatureKey, GemKnowledge, KnowEvent, LinkDir, TownLink};
use super::labels::{
    LabelDraw, LabelPlan, LabelText, QUESTION_COLOUR, dungeon_label_plan, dungeon_title_tags,
    outdoor_label_plan, town_label_plan,
};
use super::layout::DungeonLayout;
use super::raster::{GemImage, MapArt, dungeon_panel_levels, render_gem_map};
use super::seen::side_window;
use super::shown::{GemShown, GemViewState, TownPart};
use crate::memory::DUNGEON_LEVEL_COUNT;
use crate::perceived::{perceived_byte, perceived_dungeon_cell};

/// What the audit found in one view.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SpoilerReport {
    /// Every rule broken, one line each. Empty means the view tells
    /// nothing the player has not seen.
    pub violations: Vec<String>,
    /// Every word the view draws: labels and dungeon titles.
    pub texts: Vec<String>,
    /// How many "?" features and discovered features the view has.
    pub questions: usize,
    pub known: usize,
}

/// Whether `tile` may carry a feature of `category` (10.2.7's table).
pub fn kind_tile(category: FeatureCategory, tile: u8, glyphs: GlyphPolicy) -> bool {
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
        FeatureCategory::Exit => foot_terrain_accepts(tile) || is_town_door_tile(tile),
        FeatureCategory::Ladder => (0x10..=0x3F).contains(&tile),
        FeatureCategory::Fountain => tile >> 4 == 0x5,
        FeatureCategory::Chest => tile >> 4 == 0x4,
        FeatureCategory::Trap => {
            matches!(tile, 0x60 | 0x68) || (glyphs == GlyphPolicy::GemFaithful && tile >> 4 == 6)
        }
    }
}

/// The drawn tile under a feature, as the shown set has it.
fn drawn_tile(shown: &GemShown, cell: FeatureCell) -> Option<u8> {
    match (shown, cell) {
        (GemShown::Outdoor(s), FeatureCell::Outdoor { dx, dy }) => s.tile(dx, dy),
        (GemShown::Town(s), FeatureCell::Town { fx, fy, .. }) => s.tile(fx, fy),
        (GemShown::Dungeon(s), FeatureCell::Dungeon { level, x, y }) => {
            s.cell(level, x, y).map(|c| c.drawn)
        }
        _ => None,
    }
}

/// The label plan the renderer draws for `view` (its marks and words).
pub fn view_label_plan(view: &GemViewState, features: &[GemFeature], stack: bool) -> LabelPlan {
    match &view.shown {
        GemShown::Empty => LabelPlan::default(),
        GemShown::Outdoor(shown) => outdoor_label_plan(shown, features),
        GemShown::Town(shown) => town_label_plan(shown, features),
        GemShown::Dungeon(shown) => DungeonLayout::new(view.canvas, stack)
            .map(|layout| {
                let panels = dungeon_panel_levels(shown, &layout, view.page);
                dungeon_label_plan(shown, &layout, &panels, features)
            })
            .unwrap_or_default(),
    }
}

/// The dungeon titles the renderer draws for `view`.
fn view_titles(view: &GemViewState, stack: bool) -> Vec<String> {
    let GemShown::Dungeon(shown) = &view.shown else {
        return Vec::new();
    };
    DungeonLayout::new(view.canvas, stack)
        .map(|layout| {
            let panels = dungeon_panel_levels(shown, &layout, view.page);
            dungeon_title_tags(shown, &layout, &panels)
                .into_iter()
                .map(|(text, ..)| text)
                .collect()
        })
        .unwrap_or_default()
}

/// Audits one open view (see the module notes). `art` is what the window
/// draws with; it is used for the picture checks.
pub fn audit_gem_view(
    state: &PlayState,
    know: &GemKnowledge,
    view: &GemViewState,
    t: &GemMapToggles,
    art: &MapArt,
) -> SpoilerReport {
    let mut report = SpoilerReport::default();
    let mut bad = |line: String| report.violations.push(line);
    let glyphs = t.opts.glyphs;
    let features = gem_features(&view.shown, know);

    // Features: a "?" says only "something is drawn here".
    for feature in &features {
        let tile = drawn_tile(&view.shown, feature.cell);
        if feature.known() {
            report.known += 1;
            if feature.ring_colour() != feature.kind_for_audit().colour() {
                bad(format!(
                    "{:?}: a known ring not in its colour",
                    feature.cell
                ));
            }
            continue;
        }
        report.questions += 1;
        if feature.label().is_some() || feature.category().is_some() {
            bad(format!(
                "{:?}: an undiscovered feature tells its kind",
                feature.cell
            ));
        }
        if feature.ring_colour() != NEUTRAL_GREY {
            bad(format!("{:?}: a \"?\" ring not neutral grey", feature.cell));
        }
        if !feature.drawn || !tile.is_some_and(|t| kind_tile(feature.kind_for_audit(), t, glyphs)) {
            bad(format!(
                "{:?}: a \"?\" on a cell whose drawn tile {tile:02x?} is not its kind's",
                feature.cell
            ));
        }
        if feature.kind_for_audit() == FeatureCategory::Moongate && tile != Some(MOONGATE_TILE) {
            bad(format!(
                "{:?}: a moongate mark without the gate",
                feature.cell
            ));
        }
    }

    // Marks and words, as drawn.
    let stack = t.opts.dungeon_stack;
    let plan = view_label_plan(view, &features, stack);
    let mut known_words: Vec<&'static str> =
        features.iter().filter_map(GemFeature::label).collect();
    known_words.sort_unstable();
    for (items, _, _) in &plan.groups {
        for item in items {
            match item.text {
                LabelText::Word(word) => {
                    if known_words.binary_search(&word).is_err() {
                        bad(format!("the word `{word}` names no discovered feature"));
                    }
                }
                LabelText::Question => {
                    if item.colour != QUESTION_COLOUR {
                        bad(format!("a \"?\" item coloured {:?}", item.colour));
                    }
                }
                LabelText::MarkOnly => {}
            }
        }
        // The placement order (L2): every "?" after every word, and the
        // "?" by distance alone, whatever their (hidden) kinds would rank.
        let order = |items: &[super::labels::LabelItem]| {
            let mut order: Vec<usize> = (0..items.len()).collect();
            order.sort_by_key(|&i| (items[i].order_key(), i));
            order
        };
        let mut scrambled = items.clone();
        for (k, item) in scrambled.iter_mut().enumerate() {
            if item.unknown() {
                item.rank = (k as u8).wrapping_mul(37);
            }
        }
        let placed = order(items);
        if placed != order(&scrambled) {
            bad("the \"?\" order depends on more than distance".into());
        }
        let first_question = placed.iter().position(|&i| items[i].unknown());
        if let Some(first) = first_question
            && placed[first..].iter().any(|&i| !items[i].unknown())
        {
            bad("a word placed after a \"?\"".into());
        }
        let distances: Vec<u32> = placed
            .iter()
            .filter(|&&i| items[i].unknown())
            .map(|&i| items[i].distance)
            .collect();
        if distances.windows(2).any(|w| w[0] > w[1]) {
            bad("the \"?\" are not in distance order".into());
        }
    }
    let rings_known = features
        .iter()
        .filter(|f| f.known())
        .map(GemFeature::ring_colour)
        .collect::<Vec<_>>();
    for draw in &plan.draws {
        match draw {
            LabelDraw::Text { text, .. } => report.texts.push((*text).to_string()),
            LabelDraw::Ring { colour, .. } | LabelDraw::Leader { colour, .. } => {
                if *colour != QUESTION_COLOUR && !rings_known.contains(colour) {
                    bad(format!(
                        "a ring or leader in {colour:?}, no known feature's"
                    ));
                }
            }
            LabelDraw::Arrow { .. } | LabelDraw::Question { .. } => {}
        }
    }
    report.texts.extend(view_titles(view, stack));
    let underworld = know.is_known(&FeatureKey::Underworld);
    for text in &report.texts {
        if text.contains("Underworld") && !underworld {
            bad(format!("`{text}` before the Underworld was visited"));
        }
    }

    match &view.shown {
        GemShown::Dungeon(shown) => audit_dungeon(shown, know, glyphs, &mut bad),
        GemShown::Town(shown) => {
            for (fx, fy, part, tile) in shown.cells() {
                if part != TownPart::Side || tile.is_none() {
                    continue;
                }
                let seen = shown.side_world(fx, fy).is_some_and(|(_, x, y)| {
                    side_window(shown.scene)
                        .and_then(|w| w.index_of(x, y))
                        .is_some_and(|index| know.side_seen(shown.scene, index))
                });
                if !seen || !t.opts.town_surroundings {
                    bad(format!("town side cell ({fx}, {fy}) drawn but never seen"));
                }
            }
            // The picture: with the marks off, every unseen side cell is
            // black, pixel for pixel.
            let mut image = GemImage::default();
            let plain = GemMapToggles {
                labels: false,
                ..*t
            };
            render_gem_map(state, know, art, &plain, view, &mut image);
            for (fx, fy, part, tile) in shown.cells() {
                if part != TownPart::Side || tile.is_some() {
                    continue;
                }
                let (x0, y0, w, h) = shown.layout.cell_rect(fx, fy);
                let lit = (0..h as i32).any(|dy| {
                    (0..w as i32).any(|dx| {
                        image
                            .pixel((x0 + dx) as u32, (y0 + dy) as u32)
                            .is_some_and(|p| p[..3] != [0, 0, 0])
                    })
                });
                if lit {
                    bad(format!("unseen town side cell ({fx}, {fy}) is not black"));
                }
            }
        }
        GemShown::Outdoor(_) | GemShown::Empty => {}
    }
    report
}

fn audit_dungeon(
    shown: &DungeonShown,
    know: &GemKnowledge,
    glyphs: GlyphPolicy,
    bad: &mut impl FnMut(String),
) {
    let count = DUNGEON_LEVEL_COUNT;
    for level in 0..count {
        for (x, y, cell) in shown.cells(level) {
            let remembered = know.remembered(shown.record, level, x, y);
            match cell.lit {
                DungeonLit::Flood if level != shown.level => {
                    bad(format!(
                        "level {level} ({x}, {y}): live cells off the party's level"
                    ));
                }
                DungeonLit::Remembered if !remembered => {
                    bad(format!("level {level} ({x}, {y}): drawn without memory"));
                }
                _ => {}
            }
            if glyphs == GlyphPolicy::Perceived {
                let raw = cell.raw;
                let hidden = raw >> 4 == 0xD || matches!(raw, 0x61 | 0x69 | 0x62);
                let found = know.perceived_cell(shown.record, level, x, y, raw)
                    != perceived_dungeon_cell(raw);
                let plain = perceived_byte(perceived_dungeon_cell(raw), raw);
                if hidden && !found && cell.drawn != plain {
                    bad(format!(
                        "level {level} ({x}, {y}): {raw:02x} drawn as {:02x}, not as perceived",
                        cell.drawn
                    ));
                }
            }
            if shown.linked(level, x, y) {
                let other = |l: Option<u8>| l.and_then(|l| shown.cell(l, x, y)).is_some();
                if !other(level.checked_add(1)) && !other(level.checked_sub(1)) {
                    bad(format!("level {level} ({x}, {y}): a link box with one end"));
                }
            }
        }
    }
    // Panels only with evidence.
    let leads_to = |target: i32| {
        (0..count).any(|level| {
            shown.has_cells(level)
                && shown.cells(level).any(|(_, _, cell)| {
                    let (up, down) = leads(cell.drawn);
                    (up && i32::from(level) - 1 == target)
                        || (down && i32::from(level) + 1 == target)
                })
        })
    };
    for level in -1..=i32::from(count) {
        match shown.slot(level) {
            DungeonSlot::Level { level: l, cells } => {
                if cells != shown.has_cells(l) {
                    bad(format!("level {l}: a panel whose cells disagree"));
                }
                if !cells && !leads_to(i32::from(l)) {
                    bad(format!("level {l}: a black panel nothing leads to"));
                }
            }
            DungeonSlot::Underworld { named } => {
                if !leads_to(i32::from(count)) {
                    bad("an Underworld panel nothing leads to".into());
                }
                if named && !know.is_known(&FeatureKey::Underworld) {
                    bad("the Underworld named before a visit".into());
                }
            }
            DungeonSlot::Surface | DungeonSlot::Empty => {}
        }
    }
}

/// The event that sets `key`'s record bit, for [`flip_changes`].
pub fn event_for_key(key: FeatureKey) -> Option<KnowEvent> {
    Some(match key {
        FeatureKey::Place { scene } => KnowEvent::PlaceEntered { scene },
        FeatureKey::Entrance { record } => KnowEvent::EntranceNamed { record },
        FeatureKey::Shrine { virtue } => KnowEvent::ShrineEntered { virtue },
        FeatureKey::Codex => KnowEvent::CodexEntered,
        FeatureKey::Underworld => KnowEvent::UnderworldVisited,
        FeatureKey::Gate(cell) => KnowEvent::GateUsed(cell),
        FeatureKey::Sign(cell) => KnowEvent::SignRead(cell),
        FeatureKey::Trade { scene, trade } => KnowEvent::ShopOpened {
            scene,
            trade,
            keeper: None,
        },
        FeatureKey::TownLink { scene, floor, x, y } => KnowEvent::TownLinkUsed(TownLink {
            scene,
            floor,
            x,
            y,
            dir: LinkDir::Up,
        }),
        FeatureKey::Exit(run) => KnowEvent::TownExit(run),
        FeatureKey::DungeonMark(mark) => KnowEvent::DungeonMarked(mark),
    })
}

/// For each feature of `view` with a record key not yet known: sets that
/// one bit and lists every feature whose mark or label then changes
/// though it has another key (10.6: "flipping one feature's record bit
/// changes only that feature's mark and label"). Empty means none.
pub fn flip_changes(view: &GemViewState, know: &GemKnowledge) -> Vec<String> {
    let before = gem_features(&view.shown, know);
    let mut out = Vec::new();
    let mut keys: Vec<FeatureKey> = before
        .iter()
        .filter(|f| !f.known())
        .filter_map(GemFeature::key)
        .collect();
    keys.dedup();
    for key in keys {
        let Some(event) = event_for_key(key) else {
            continue;
        };
        let mut flipped = know.clone();
        flipped.apply(&event);
        let after = gem_features(&view.shown, &flipped);
        let changed_other = before.iter().any(|a| {
            a.key() != Some(key)
                && !after
                    .iter()
                    .any(|b| b.cell == a.cell && b.key() == a.key() && b.known() == a.known())
        });
        let added = after
            .iter()
            .any(|b| b.key() != Some(key) && !before.iter().any(|a| a == b));
        if changed_other || added {
            out.push(format!("flipping {key:?} changed another feature"));
        }
    }
    out
}

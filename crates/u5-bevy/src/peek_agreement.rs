//! Test-only recorder for the ring peek's agreement test (Phase B M1-05,
//! design 2.3.6 and 2.6).
//!
//! The grow-left layout is used exactly when
//! `PlayState::top_down_world_view_active` is true (2.2.8), so that
//! predicate has to agree with the branch the faithful frame render really
//! takes. The headless play-frame compositor notes, for every frame, the
//! predicate on the state it was handed and which branch drew the
//! viewport; the report writer files those notes under the report's
//! label. A test then runs the frame and route suites and compares the
//! two on every state they produce. Nothing here is compiled outside
//! tests, and the recorder is off unless a test turns it on.
//!
//! Phase B M2-02 (design 9.3.7) pins `PlayState::map_view_kind` the same
//! way: each frame notes the kind, and [`FrameNote::kind_shown`] maps the
//! branch that drew the viewport to the kind it showed.
//!
//! Phase B M2-05 (design 9.3.7) checks the object-cell peek the same way:
//! after each map or combat render, every cell the peek names must hold
//! exactly the atlas tile it says the render drew there
//! ([`note_object_cells`]).
//!
//! Phase B M3-04 (design 4.3.3, 4.6) pins the dungeon corridor iterator
//! the same way: on every dungeon frame, the cells
//! `u5_automap::dungeon_corridor_cells` names must be exactly the cells
//! whose content the faithful corridor render depends on, found by
//! rendering clones with each cell of the level set to passage and to wall
//! ([`note_corridor_cells`]).

use std::cell::RefCell;
use std::collections::HashSet;

use u5_runtime::enhancements::SpriteMasks;
use u5_runtime::{
    Area, DUNGEON_BANDS, MapViewKind, PlayState, TILE_ATLAS_SIDE, TILE_ATLAS_TILE_PIXELS,
    TileAtlas, TileViewport, VIEWPORT_PLAYER_ROW, blit_terrain_tile_to_viewport,
    dungeon_cell_index,
};

/// The branch `render_base_framebuffer` took for the viewport.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BaseBranch {
    BlackthornRescue,
    BlackthornAudience,
    ShrinePresentation,
    Combat,
    Dungeon,
    VisibilitySweep,
    Map,
    TextPanel,
}

/// One composed play frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FrameNote {
    /// The predicate on the state as the compositor received it.
    pub predicate: bool,
    /// The predicate once the frame was drawn (the render advances a
    /// visibility sweep, so the two can differ on the sweep's last frame).
    pub predicate_after: bool,
    /// The world view, not the endgame tableau or the absorption arena.
    pub world_viewport: bool,
    /// The branch that drew the viewport, if the base render ran.
    pub branch: Option<BaseBranch>,
    /// A view overlay was drawn over the viewport.
    pub overlay: bool,
    /// The endgame sequence was running.
    pub endgame: bool,
    /// `map_view_kind` on the state as the compositor received it.
    pub kind: Option<MapViewKind>,
    /// `map_view_kind` once the frame was drawn.
    pub kind_after: Option<MapViewKind>,
    /// The state's area was a town (the map branch's town or world).
    pub town_area: bool,
}

impl FrameNote {
    /// What the frame showed: the plain top-down map view, with nothing
    /// over it and no fixed screen in charge (design 2.2.8).
    pub fn showed_top_down_view(self) -> bool {
        self.world_viewport
            && self.branch == Some(BaseBranch::Map)
            && !self.overlay
            && !self.endgame
    }

    /// The map the frame showed, from the branch that drew it (design
    /// 9.3.7): the map branch is the world or a town, combat and the
    /// dungeon are themselves, and the text panel, the sweep and the three
    /// cinematic branches show no map. A view overlay, the endgame tableau
    /// or absorption arena, or the endgame over the world shows none
    /// either.
    pub fn kind_shown(self) -> Option<MapViewKind> {
        if !self.world_viewport || self.overlay || self.endgame {
            return None;
        }
        match self.branch? {
            BaseBranch::Map if self.town_area => Some(MapViewKind::Town),
            BaseBranch::Map => Some(MapViewKind::World),
            BaseBranch::Combat => Some(MapViewKind::Combat),
            BaseBranch::Dungeon => Some(MapViewKind::Dungeon),
            BaseBranch::TextPanel
            | BaseBranch::VisibilitySweep
            | BaseBranch::BlackthornRescue
            | BaseBranch::BlackthornAudience
            | BaseBranch::ShrinePresentation => None,
        }
    }
}

/// The object-cell peek checked against the frames (M2-05).
#[derive(Clone, Debug, Default)]
pub(crate) struct ObjectCellTally {
    /// Map and combat frames peeked.
    pub frames: usize,
    /// Frames whose peek named at least one cell.
    pub with_cells: usize,
    /// Cells compared with the frame.
    pub cells: usize,
    /// Cells whose `under` animates.
    pub animated: usize,
    /// One line per cell whose pixels were not the `drawn` tile's.
    pub mismatches: Vec<String>,
}

/// The corridor iterator checked against the dungeon frames (M3-04).
#[derive(Clone, Debug, Default)]
pub(crate) struct CorridorTally {
    /// Dungeon frames seen.
    pub frames: usize,
    /// Distinct views (level bytes, position, facing, light) flip-checked.
    pub views: usize,
    /// Views with a personal light.
    pub lit: usize,
    /// Views whose corridor stopped at a blocked band before the last.
    pub blocked: usize,
    /// Views with a heavy door underfoot (the point-blank rule).
    pub point_blank: usize,
    /// Cells the iterator named, over the distinct views.
    pub cells: usize,
    /// One line per view whose named cells were not the frame's.
    pub mismatches: Vec<String>,
}

#[derive(Default)]
struct Recorder {
    branch: Option<BaseBranch>,
    pending: Vec<FrameNote>,
    filed: Vec<(String, Vec<FrameNote>)>,
    object_cells: ObjectCellTally,
    /// The masks of the last atlas seen, by its pixel buffer's address.
    masks: Option<(usize, SpriteMasks)>,
    corridor: CorridorTally,
    corridor_seen: HashSet<Vec<u8>>,
}

thread_local! {
    static RECORDER: RefCell<Option<Recorder>> = const { RefCell::new(None) };
}

/// Start recording on this thread, dropping anything recorded before.
pub(crate) fn start() {
    RECORDER.with(|cell| *cell.borrow_mut() = Some(Recorder::default()));
}

/// Stop recording and return every report label with its frames.
pub(crate) fn finish() -> Vec<(String, Vec<FrameNote>)> {
    RECORDER.with(|cell| {
        cell.borrow_mut()
            .take()
            .map(|r| r.filed)
            .unwrap_or_default()
    })
}

/// The object-cell tally since [`start`].
pub(crate) fn object_cell_tally() -> ObjectCellTally {
    RECORDER.with(|cell| {
        cell.borrow()
            .as_ref()
            .map(|recorder| recorder.object_cells.clone())
            .unwrap_or_default()
    })
}

/// The corridor tally since [`start`].
pub(crate) fn corridor_tally() -> CorridorTally {
    RECORDER.with(|cell| {
        cell.borrow()
            .as_ref()
            .map(|recorder| recorder.corridor.clone())
            .unwrap_or_default()
    })
}

/// Check the corridor iterator on the dungeon frame just drawn from
/// `state` (design 4.6, "Corridor iterator, black-box"): a cell of the
/// level is visible exactly when the faithful corridor render, on a clone,
/// differs between the cell set to passage `0x00` and to wall `0xB0`, every
/// other cell held. The visible set must be the iterator's, or, unlit, the
/// render must paint nothing and the iterator name the party's cell alone.
/// Each distinct view is checked once; frames without corridor art are
/// counted but not checked.
pub(crate) fn note_corridor_cells(state: &PlayState, atlas: &TileAtlas) {
    let Some(banks) = atlas.dungeon_billboards.as_ref() else {
        return;
    };
    let Area::Dungeon { level, .. } = state.area else {
        return;
    };
    let Some(cells) = u5_automap::dungeon_corridor_cells(state) else {
        return;
    };
    RECORDER.with(|cell| {
        let mut borrow = cell.borrow_mut();
        let Some(recorder) = borrow.as_mut() else {
            return;
        };
        recorder.corridor.frames += 1;
        let at = |x: u8, y: u8| dungeon_cell_index(level, usize::from(x), usize::from(y));
        let mut key: Vec<u8> = (0..8u8)
            .flat_map(|y| (0..8u8).map(move |x| (x, y)))
            .map(|(x, y)| state.grid[at(x, y)])
            .collect();
        key.extend([
            cells.record,
            level,
            state.player.x as u8,
            state.player.y as u8,
            state.player.facing as u8,
            u8::from(cells.lit),
        ]);
        if !recorder.corridor_seen.insert(key) {
            return;
        }
        let tally = &mut recorder.corridor;
        tally.views += 1;
        tally.lit += usize::from(cells.lit);
        tally.cells += cells.cells.len();
        let last_band = cells.cells.last().map_or(0, |c| c.band);
        tally.blocked += usize::from(cells.lit && usize::from(last_band) + 1 < DUNGEON_BANDS);
        let (px, py) = (cells.cells[0].x, cells.cells[0].y);
        tally.point_blank += usize::from(cells.lit && state.grid[at(px, py)] >> 4 == 0xE);

        let render = |probe: &PlayState| {
            probe
                .clone()
                .render_dungeon_viewport(
                    VIEWPORT_PLAYER_ROW,
                    atlas.depth,
                    Some(banks),
                    atlas.dungeon_sprites.as_ref(),
                )
                .map(|viewport| viewport.pixels)
        };
        let mut visible = Vec::new();
        for y in 0..8u8 {
            for x in 0..8u8 {
                let mut open = state.clone();
                open.grid[at(x, y)] = 0x00;
                let mut shut = state.clone();
                shut.grid[at(x, y)] = 0xB0;
                match (render(&open), render(&shut)) {
                    (Ok(a), Ok(b)) if a != b => visible.push((x, y)),
                    (Ok(_), Ok(_)) => {}
                    (Err(err), _) | (_, Err(err)) => {
                        tally.mismatches.push(format!("render failed: {err}"));
                        return;
                    }
                }
            }
        }
        let mut named: Vec<(u8, u8)> = if cells.lit {
            cells.positions().collect()
        } else {
            Vec::new()
        };
        named.sort_unstable();
        visible.sort_unstable();
        let unlit_wrong = !cells.lit && cells.cells.len() != 1;
        if named != visible || unlit_wrong {
            tally.mismatches.push(format!(
                "record {} level {level} at ({}, {}) facing {:?} lit {}: iterator {named:?}, \
                 frame {visible:?}",
                cells.record, state.player.x, state.player.y, state.player.facing, cells.lit
            ));
        }
    });
}

/// Peek the object cells of the frame `viewport` just drawn from `state`
/// and compare each named cell with the atlas tile it says was drawn
/// (design 9.3.7: `drawn` is the faithful render's cell). Only for frames
/// the map or combat branch drew; the cursor's white ring, painted over a
/// combat cell after the sprite, is not compared.
pub(crate) fn note_object_cells(state: &PlayState, atlas: &TileAtlas, viewport: &TileViewport) {
    RECORDER.with(|cell| {
        let mut borrow = cell.borrow_mut();
        let Some(recorder) = borrow.as_mut() else {
            return;
        };
        let key = atlas.pixels.as_ptr() as usize;
        if recorder.masks.as_ref().is_none_or(|(seen, _)| *seen != key) {
            recorder.masks = Some((key, SpriteMasks::from_atlas(atlas)));
        }
        let masks = &recorder.masks.as_ref().expect("built above").1;
        let tally = &mut recorder.object_cells;
        let Some(peek) = state.peek_object_cells(atlas, masks) else {
            return;
        };
        tally.frames += 1;
        tally.with_cells += usize::from(!peek.cells.is_empty());
        let cursor = state.combat_overlay_draw_cells().cursor_draw_cell;
        for object in &peek.cells {
            tally.cells += 1;
            tally.animated += usize::from(peek.animated_under);
            let mut tile = TileViewport {
                depth: atlas.depth,
                cells_wide: 1,
                cells_high: 1,
                width: TILE_ATLAS_SIDE,
                height: TILE_ATLAS_SIDE,
                pixels: vec![0; TILE_ATLAS_TILE_PIXELS],
            };
            if let Err(err) = blit_terrain_tile_to_viewport(
                &mut tile,
                atlas,
                usize::from(object.drawn),
                0,
                0,
                state.water_scroll,
                &state.fire_flicker,
            ) {
                tally.mismatches.push(format!("{object:?}: {err}"));
                continue;
            }
            let ring = [0, 1, 14, 15];
            let on_cursor = cursor == Some((object.col, object.row));
            let differ = (0..TILE_ATLAS_SIDE)
                .flat_map(|y| (0..TILE_ATLAS_SIDE).map(move |x| (x, y)))
                .filter(|&(x, y)| !(on_cursor && (ring.contains(&x) || ring.contains(&y))))
                .filter(|&(x, y)| {
                    let at = (usize::from(object.row) * TILE_ATLAS_SIDE + y) * viewport.width
                        + usize::from(object.col) * TILE_ATLAS_SIDE
                        + x;
                    viewport.pixels.get(at) != Some(&tile.pixels[y * TILE_ATLAS_SIDE + x])
                })
                .count();
            if differ > 0 {
                tally.mismatches.push(format!(
                    "cell ({}, {}) drawn {} under {:#04x}: {differ} pixels differ",
                    object.col, object.row, object.drawn, object.under
                ));
            }
        }
    });
}

/// Note the branch the base render took.
pub(crate) fn note_branch(branch: BaseBranch) {
    RECORDER.with(|cell| {
        if let Some(recorder) = cell.borrow_mut().as_mut() {
            recorder.branch = Some(branch);
        }
    });
}

/// Forget any branch noted before this frame's render.
pub(crate) fn begin_frame() {
    RECORDER.with(|cell| {
        if let Some(recorder) = cell.borrow_mut().as_mut() {
            recorder.branch = None;
        }
    });
}

/// The frame's state, as the compositor saw it before and after the
/// render.
pub(crate) struct FrameState {
    pub predicate: bool,
    pub predicate_after: bool,
    pub world_viewport: bool,
    pub overlay: bool,
    pub endgame: bool,
    pub kind: Option<MapViewKind>,
    pub kind_after: Option<MapViewKind>,
    pub town_area: bool,
}

/// Note one composed frame, taking the branch noted since
/// [`begin_frame`].
pub(crate) fn note_frame(frame: FrameState) {
    let FrameState {
        predicate,
        predicate_after,
        world_viewport,
        overlay,
        endgame,
        kind,
        kind_after,
        town_area,
    } = frame;
    RECORDER.with(|cell| {
        if let Some(recorder) = cell.borrow_mut().as_mut() {
            let branch = recorder.branch.take();
            recorder.pending.push(FrameNote {
                predicate,
                predicate_after,
                world_viewport,
                branch,
                overlay,
                endgame,
                kind,
                kind_after,
                town_area,
            });
        }
    });
}

/// File the frames composed since the last report under `label`.
pub(crate) fn note_report(label: &str) {
    RECORDER.with(|cell| {
        if let Some(recorder) = cell.borrow_mut().as_mut() {
            let frames = std::mem::take(&mut recorder.pending);
            recorder.filed.push((label.to_string(), frames));
        }
    });
}

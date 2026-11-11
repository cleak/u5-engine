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

use std::cell::RefCell;

use u5_runtime::MapViewKind;

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

#[derive(Default)]
struct Recorder {
    branch: Option<BaseBranch>,
    pending: Vec<FrameNote>,
    filed: Vec<(String, Vec<FrameNote>)>,
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

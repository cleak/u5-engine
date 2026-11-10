//! The side-space arbiter and the panel host (Phase B design 1.3.8).
//!
//! A window wider than 4:3 leaves bars beside the frame. Chapter 5's
//! journal panel and chapter 8's scrollback panel want them; each registers
//! a [`SidePanelProvider`] in the [`PanelRegistry`] while its toggle is on
//! and removes it when the toggle goes off.
//!
//! - `layout_side_space`, in [`EnhSet::Layout`], is the only layout
//!   solver ([`solve_side_space`]). It works out the frame scale with no
//!   panels (`u0`), tries the claimed panels, and accepts them while the
//!   frame keeps at least `display.panel_guard` percent of `u0`; otherwise
//!   it drops the lowest in `display.panel_priority` and tries again. A
//!   panel that finds no room is hidden, with a one-time toast.
//! - While no panel is granted the faithful `fit_display_to_window` owns
//!   the frame, through its one run condition, [`faithful_fit_owns_frame`].
//!   Otherwise [`SideLayout`] owns it and `fit_frame_to_side_layout` sizes
//!   and places the frame sprite. The frame size is unchanged unless a
//!   panel is granted or a [`FramePolicy`] other than the 1.0.1 Fit is in
//!   force (chapter 2's scale modes, [`solve_frame`]).
//! - The generic cell-panel renderer draws [`PanelContent::Cells`] into a
//!   136x200 image (an 8-pixel gutter on the frame side, then 16 cells of
//!   the frame's own font, a title and 24 rows), repainting only when the
//!   cells change. Panels are `bevy_ui` nodes; the frame image is never
//!   written.
//!
//! None of this is in the app until a side-claiming toggle is on
//! ([`add_side_space`]): with every toggle off no resource, node or system
//! exists, and `fit_display_to_window` runs as in 1.0.1. Panels need no
//! display or widescreen toggle.

use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use u5_runtime::enhancements::{
    FeatureToggles, PANEL_WORDS, SIDE_CLAIM_KEYS, SettingValues, Toggle,
};
use u5_runtime::{CH_CELL_SIDE, FixedCellFont, PlayState};

use super::EnhSet;
use super::toast::{Toasts, install_shell_services};
use crate::{
    DISPLAY_PIXEL_ASPECT, VISUAL_PLAY_FRAME_HEIGHT, VISUAL_PLAY_FRAME_WIDTH, VisualState,
    fitted_display_size,
};

/// The one-time toast for a panel that finds no room (1.2.8, 5.2.10).
pub const PANEL_HIDDEN: &str = "There is no room for a side panel in this window.";

/// A cell panel's image: the gutter and 16 cells across, a title and 24
/// rows down.
pub const PANEL_WIDTH: u32 = PANEL_GUTTER + (PANEL_COLUMNS * CH_CELL_SIDE) as u32;
pub const PANEL_HEIGHT: u32 = (PANEL_TEXT_ROWS * CH_CELL_SIDE) as u32;
/// The blank strip on the frame side of a cell panel.
pub const PANEL_GUTTER: u32 = 8;
/// Cells per panel row.
pub const PANEL_COLUMNS: usize = 16;
/// Rows below the title.
pub const PANEL_BODY_ROWS: usize = 24;
const PANEL_TEXT_ROWS: usize = PANEL_BODY_ROWS + 1;

const PANEL_GUARD_KEY: &str = "display.panel_guard";
const PANEL_PRIORITY_KEY: &str = "display.panel_priority";
/// `display.panel_guard`'s default and range, in percent.
const DEFAULT_GUARD: u8 = 80;
const GUARD_RANGE: std::ops::RangeInclusive<u8> = 50..=100;

/// White title, light grey rows, on black: the text window's own pens.
const TITLE_INK: [u8; 4] = [0xff, 0xff, 0xff, 0xff];
const ROW_INK: [u8; 4] = [0xaa, 0xaa, 0xaa, 0xff];
const PAPER: [u8; 4] = [0x00, 0x00, 0x00, 0xff];

/// A panel kind, in its default priority order. The minimap (chapter 4)
/// is parked (U18); it never claims space from a cell panel.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PanelKind {
    Journal,
    Conversation,
    Minimap,
}

impl PanelKind {
    pub const ALL: [PanelKind; 3] = [
        PanelKind::Journal,
        PanelKind::Conversation,
        PanelKind::Minimap,
    ];

    /// The word `display.panel_priority` uses for it. The minimap has
    /// none: it is always last.
    pub fn word(self) -> Option<&'static str> {
        match self {
            PanelKind::Journal => Some(PANEL_WORDS[0]),
            PanelKind::Conversation => Some(PANEL_WORDS[1]),
            PanelKind::Minimap => None,
        }
    }

    pub fn from_word(word: &str) -> Option<PanelKind> {
        PanelKind::ALL
            .into_iter()
            .find(|kind| kind.word() == Some(word))
    }

    /// Whether it is a cell panel, which takes a bar of its own.
    pub fn is_cell_panel(self) -> bool {
        self != PanelKind::Minimap
    }
}

/// A side of the frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PanelSide {
    Left,
    Right,
}

impl PanelSide {
    pub fn other(self) -> PanelSide {
        match self {
            PanelSide::Left => PanelSide::Right,
            PanelSide::Right => PanelSide::Left,
        }
    }
}

/// The side a provider asks for (`journal.side_panel_side`, chapter 8's
/// scrollback side). `Auto` takes the side the other panel does not use.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SidePreference {
    #[default]
    Auto,
    Left,
    Right,
}

/// A cell panel's text: a title and [`PANEL_BODY_ROWS`] rows of
/// [`PANEL_COLUMNS`] IBM.CH codes. Shorter text is padded with spaces and
/// longer text is cut.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PanelRows {
    rows: Box<[[u8; PANEL_COLUMNS]; PANEL_TEXT_ROWS]>,
}

impl PanelRows {
    pub fn new(title: &str, rows: &[&str]) -> Self {
        let mut panel = Self {
            rows: Box::new([[b' '; PANEL_COLUMNS]; PANEL_TEXT_ROWS]),
        };
        panel.set(0, title);
        for (index, row) in rows.iter().take(PANEL_BODY_ROWS).enumerate() {
            panel.set(index + 1, row);
        }
        panel
    }

    fn set(&mut self, row: usize, text: &str) {
        for (cell, byte) in self.rows[row].iter_mut().zip(text.bytes()) {
            *cell = byte;
        }
    }

    pub fn title(&self) -> &[u8; PANEL_COLUMNS] {
        &self.rows[0]
    }

    /// Body row `index`, 0 to 23.
    pub fn row(&self, index: usize) -> &[u8; PANEL_COLUMNS] {
        &self.rows[index + 1]
    }
}

/// What a provider shows.
#[derive(Clone, Debug, PartialEq)]
pub enum PanelContent {
    /// Text, drawn by the generic cell-panel renderer.
    Cells(PanelRows),
    /// A finished image (the minimap only).
    Image(Handle<Image>),
}

/// A side panel's content source. Chapter 5 supplies the journal's,
/// chapter 8 the scrollback's.
pub trait SidePanelProvider: Send + Sync + 'static {
    fn kind(&self) -> PanelKind;

    /// The side it asks for.
    fn side(&self) -> SidePreference {
        SidePreference::Auto
    }

    /// What to show now. `None` (no game yet, or its page is open) leaves
    /// the slot empty; the panel keeps its place.
    fn snapshot(&self, state: Option<&PlayState>) -> Option<PanelContent>;
}

/// The providers whose features are on. A provider in the registry is a
/// claim on side space.
#[derive(Resource, Default)]
pub struct PanelRegistry(Vec<Box<dyn SidePanelProvider>>);

impl PanelRegistry {
    /// Adds `provider`, replacing one of the same kind.
    pub fn register(&mut self, provider: impl SidePanelProvider) {
        self.remove(provider.kind());
        self.0.push(Box::new(provider));
    }

    /// Removes the provider of `kind`, when its feature goes off.
    pub fn remove(&mut self, kind: PanelKind) {
        self.0.retain(|provider| provider.kind() != kind);
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    fn get(&self, kind: PanelKind) -> Option<&dyn SidePanelProvider> {
        self.0
            .iter()
            .find(|provider| provider.kind() == kind)
            .map(|provider| &**provider)
    }

    /// The claims, in registration order.
    pub fn claims(&self) -> Vec<PanelClaim> {
        self.0
            .iter()
            .map(|provider| PanelClaim {
                kind: provider.kind(),
                side: provider.side(),
            })
            .collect()
    }
}

/// One panel asking for a bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PanelClaim {
    pub kind: PanelKind,
    pub side: SidePreference,
}

/// Where everything goes, in logical window pixels from the top-left
/// corner. `frame` is `Some` exactly while the arbiter owns the frame
/// sprite, which is while a panel is granted or a scale policy places it.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct SideLayout {
    /// The bars beside the frame, where there are any.
    pub left: Option<Rect>,
    pub right: Option<Rect>,
    pub frame: Option<Rect>,
    pub granted: Vec<(PanelKind, Rect)>,
}

impl SideLayout {
    /// Whether the arbiter, not `fit_display_to_window`, sizes the frame.
    pub fn owns_frame(&self) -> bool {
        self.frame.is_some()
    }

    /// The side `kind` was granted, if it was.
    pub fn side_of(&self, kind: PanelKind) -> Option<PanelSide> {
        let frame = self.frame?;
        self.granted
            .iter()
            .find(|(granted, _)| *granted == kind)
            .map(|(_, rect)| {
                if rect.max.x <= frame.min.x {
                    PanelSide::Left
                } else {
                    PanelSide::Right
                }
            })
    }
}

/// The arbiter's settings: `display.panel_priority` and
/// `display.panel_guard`.
#[derive(Resource, Clone, Debug, PartialEq, Eq)]
pub struct SideSpaceSettings {
    /// Every kind, highest priority first.
    pub priority: Vec<PanelKind>,
    /// The smallest frame scale a panel may leave, in percent of the scale
    /// with no panel.
    pub guard_percent: u8,
}

impl Default for SideSpaceSettings {
    fn default() -> Self {
        Self::from_priority(PANEL_WORDS.iter().copied(), DEFAULT_GUARD)
    }
}

impl SideSpaceSettings {
    /// From the resolved values. A kind the list leaves out goes after the
    /// ones it names, in the default order; the minimap is always last.
    pub fn from_values(values: &SettingValues) -> Self {
        let guard = values
            .int(PANEL_GUARD_KEY)
            .and_then(|percent| u8::try_from(percent).ok())
            .filter(|percent| GUARD_RANGE.contains(percent))
            .unwrap_or(DEFAULT_GUARD);
        match values.list(PANEL_PRIORITY_KEY) {
            Some(words) => Self::from_priority(words.iter().map(String::as_str), guard),
            None => Self::from_priority(PANEL_WORDS.iter().copied(), guard),
        }
    }

    fn from_priority<'a>(words: impl Iterator<Item = &'a str>, guard_percent: u8) -> Self {
        let mut priority = Vec::with_capacity(PanelKind::ALL.len());
        for kind in words.filter_map(PanelKind::from_word).chain(PanelKind::ALL) {
            if !priority.contains(&kind) {
                priority.push(kind);
            }
        }
        Self {
            priority,
            guard_percent,
        }
    }

    fn rank(&self, kind: PanelKind) -> usize {
        self.priority
            .iter()
            .position(|k| *k == kind)
            .unwrap_or(usize::MAX)
    }
}

/// What the solver decided.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SideSolution {
    pub layout: SideLayout,
    /// Claimed cell panels that got no room, highest priority first.
    pub hidden: Vec<PanelKind>,
}

/// The window as the frame fit sees it: its size in physical pixels and
/// the display scale (physical pixels per logical pixel).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WindowPixels {
    pub physical: UVec2,
    pub scale_factor: f32,
}

impl WindowPixels {
    pub fn of(window: &Window) -> Self {
        Self {
            physical: window.resolution.physical_size(),
            scale_factor: window.resolution.scale_factor(),
        }
    }

    /// A window of `logical` pixels at a display scale of 1.
    pub fn logical_at_one(logical: Vec2) -> Self {
        Self {
            physical: logical.as_uvec2(),
            scale_factor: 1.0,
        }
    }

    pub fn logical(self) -> Vec2 {
        self.physical.as_vec2() / self.scale_factor
    }
}

/// A frame scale policy other than the foundation's Fit: chapter 2's sharp,
/// integer and stretch modes (2.3.5) supply one.
pub trait FrameFit: Send + Sync + 'static {
    /// Where an `image`-sized frame goes in `window`, in logical pixels from
    /// the top-left corner. `None` leaves the frame to the 1.0.1 fit.
    fn place(&self, window: WindowPixels, image: UVec2) -> Option<Rect>;
}

/// The frame scale policy in force. Absent (every run but one with a
/// non-Fit `display.scale_mode` or `display.pixel_aspect`) is the 1.0.1
/// Fit.
#[derive(Resource)]
pub struct FramePolicy(pub Box<dyn FrameFit>);

/// The size of the image the frame fit places, when a presentation step
/// shows something other than the 320x200 frame: chapter 2's grow-left
/// canvas (2.3.5, "the frame fit sizes the canvas"). The step's feature
/// sets it in `EnhSet::Layout`, before the layout is solved, from the same
/// test the step applies, so the sprite is sized for the image it shows on
/// the same frame. Absent, or `None`, is the 320x200 frame.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameImageSize(pub Option<UVec2>);

impl FrameImageSize {
    /// The image the frame fit places.
    pub fn image(size: Option<&FrameImageSize>) -> UVec2 {
        size.and_then(|size| size.0).unwrap_or(UVec2::new(
            VISUAL_PLAY_FRAME_WIDTH,
            VISUAL_PLAY_FRAME_HEIGHT,
        ))
    }
}

/// The layout solver (1.3.8). `window` is the window's logical size,
/// `image` the presented image's (the 320x200 frame in the foundation),
/// both under the Fit policy and the 1.20 pixel aspect.
///
/// 1. `u0` is the frame scale with no panel.
/// 2. Each claimed cell panel gets a side: in priority order, those asking
///    for a side take it if it is free; then the rest take a free side
///    (right first for `Auto`, the other side for a panel that lost its
///    own). One cell panel per bar; a panel with no side is hidden.
/// 3. The granted panels and the frame share the window at scale `u`. The
///    set is accepted while `u >= panel_guard * u0`; otherwise the lowest
///    in priority is hidden and the rest tried again.
/// 4. When `u` is `u0` and the centred frame leaves room in its bars, the
///    frame stays where `fit_display_to_window` puts it and the panels sit
///    against it; otherwise frame and panels are centred as one group.
///
/// The minimap never claims space from a cell panel; while it is parked
/// (U18) it is never granted.
pub fn solve_side_space(
    window: Vec2,
    image: UVec2,
    claims: &[PanelClaim],
    settings: &SideSpaceSettings,
) -> SideSolution {
    if window.x <= 0.0 || window.y <= 0.0 || image.x == 0 || image.y == 0 {
        // A minimised window: nothing to arrange, nothing to report.
        return SideSolution::default();
    }
    let (mut placed, mut hidden) = assign_sides(claims, settings);

    // 3. The guard.
    let image_size = Vec2::new(image.x as f32, image.y as f32 * DISPLAY_PIXEL_ASPECT);
    let scale = |panels: usize| {
        let width = image_size.x + (panels as u32 * PANEL_WIDTH) as f32;
        (window.x / width).min(window.y / image_size.y)
    };
    let u0 = scale(0);
    let guard = f32::from(settings.guard_percent) / 100.0;
    let mut u = scale(placed.len());
    while !placed.is_empty() && u < guard * u0 {
        let (dropped, _) = placed.pop().expect("not empty");
        hidden.push(dropped);
        u = scale(placed.len());
    }
    hidden.sort_by_key(|kind| settings.rank(*kind));
    if placed.is_empty() {
        return SideSolution {
            layout: SideLayout::default(),
            hidden,
        };
    }

    // 4. Geometry.
    let has = |side: PanelSide| placed.iter().any(|(_, s)| *s == side);
    let unshrunk = u >= u0;
    let frame_size = if unshrunk {
        fitted_display_size(image.x, image.y, window)
    } else {
        image_size * u
    };
    let panel_size = Vec2::new(
        PANEL_WIDTH as f32 * frame_size.x / image.x as f32,
        PANEL_HEIGHT as f32 * frame_size.y / image.y as f32,
    );
    let centred = (window.x - frame_size.x) / 2.0;
    let room = |side| !has(side) || centred >= panel_size.x;
    let frame_left = if unshrunk && room(PanelSide::Left) && room(PanelSide::Right) {
        centred
    } else {
        let group = frame_size.x + panel_size.x * placed.len() as f32;
        let left_panel = if has(PanelSide::Left) {
            panel_size.x
        } else {
            0.0
        };
        (window.x - group) / 2.0 + left_panel
    };
    let top = (window.y - frame_size.y) / 2.0;
    let frame = Rect::from_corners(
        Vec2::new(frame_left, top),
        Vec2::new(frame_left + frame_size.x, top + frame_size.y),
    );
    SideSolution {
        layout: framed_layout(window, frame, panel_size, &placed),
        hidden,
    }
}

/// [`solve_side_space`] under a scale policy (2.3.5). With no policy, or
/// one that leaves this window to the 1.0.1 fit, it is
/// [`solve_side_space`] exactly. Otherwise the policy places the frame,
/// which the arbiter then owns even with no panel; with panels, the policy
/// places the frame and its panels as one image `PANEL_WIDTH` wider per
/// panel, so an integer mode keeps whole factors for both, and the guard
/// compares the frame's horizontal scale with and without them.
pub fn solve_frame(
    window: WindowPixels,
    image: UVec2,
    claims: &[PanelClaim],
    settings: &SideSpaceSettings,
    policy: Option<&dyn FrameFit>,
) -> SideSolution {
    let logical = window.logical();
    let alone = policy.and_then(|policy| policy.place(window, image));
    let (Some(policy), Some(alone)) = (policy, alone) else {
        return solve_side_space(logical, image, claims, settings);
    };
    if logical.x <= 0.0 || logical.y <= 0.0 || image.x == 0 || image.y == 0 {
        return SideSolution::default();
    }
    let (mut placed, mut hidden) = assign_sides(claims, settings);
    let group = |panels: usize| {
        let width = image.x + panels as u32 * PANEL_WIDTH;
        let size = UVec2::new(width, image.y);
        let rect = policy.place(window, size).unwrap_or_else(|| {
            let fitted = fitted_display_size(size.x, size.y, logical);
            let min = (logical - fitted) / 2.0;
            Rect::from_corners(min, min + fitted)
        });
        (rect, rect.width() / width as f32)
    };
    let u0 = alone.width() / image.x as f32;
    let guard = f32::from(settings.guard_percent) / 100.0;
    let (mut rect, mut u) = group(placed.len());
    while !placed.is_empty() && u < guard * u0 {
        let (dropped, _) = placed.pop().expect("not empty");
        hidden.push(dropped);
        (rect, u) = group(placed.len());
    }
    hidden.sort_by_key(|kind| settings.rank(*kind));
    if placed.is_empty() {
        return SideSolution {
            layout: framed_layout(logical, alone, Vec2::ZERO, &placed),
            hidden,
        };
    }
    let panel_size = Vec2::new(PANEL_WIDTH as f32 * u, rect.height());
    let left = if placed.iter().any(|(_, side)| *side == PanelSide::Left) {
        panel_size.x
    } else {
        0.0
    };
    let min = Vec2::new(rect.min.x + left, rect.min.y);
    let frame = Rect::from_corners(min, min + Vec2::new(image.x as f32 * u, rect.height()));
    SideSolution {
        layout: framed_layout(logical, frame, panel_size, &placed),
        hidden,
    }
}

/// Step 2 of [`solve_side_space`]: the cell panels that got a side, in
/// priority order, and those that got none.
fn assign_sides(
    claims: &[PanelClaim],
    settings: &SideSpaceSettings,
) -> (Vec<(PanelKind, PanelSide)>, Vec<PanelKind>) {
    let mut cells: Vec<PanelClaim> = Vec::new();
    for claim in claims.iter().filter(|claim| claim.kind.is_cell_panel()) {
        if !cells.iter().any(|c| c.kind == claim.kind) {
            cells.push(*claim);
        }
    }
    cells.sort_by_key(|claim| settings.rank(claim.kind));

    let mut placed: Vec<(PanelKind, PanelSide)> = Vec::new();
    let mut hidden: Vec<PanelKind> = Vec::new();
    let free = |placed: &[(PanelKind, PanelSide)], side: PanelSide| {
        placed.iter().all(|(_, taken)| *taken != side)
    };
    for claim in &cells {
        let wanted = match claim.side {
            SidePreference::Left => PanelSide::Left,
            SidePreference::Right => PanelSide::Right,
            SidePreference::Auto => continue,
        };
        if free(&placed, wanted) {
            placed.push((claim.kind, wanted));
        }
    }
    for claim in &cells {
        if placed.iter().any(|(kind, _)| *kind == claim.kind) {
            continue;
        }
        let order: &[PanelSide] = match claim.side {
            SidePreference::Auto => &[PanelSide::Right, PanelSide::Left],
            SidePreference::Left => &[PanelSide::Right],
            SidePreference::Right => &[PanelSide::Left],
        };
        match order.iter().copied().find(|side| free(&placed, *side)) {
            Some(side) => placed.push((claim.kind, side)),
            None => hidden.push(claim.kind),
        }
    }
    placed.sort_by_key(|(kind, _)| settings.rank(*kind));
    (placed, hidden)
}

/// The layout for a placed frame: its panels against it and the bars
/// beside it.
fn framed_layout(
    window: Vec2,
    frame: Rect,
    panel_size: Vec2,
    placed: &[(PanelKind, PanelSide)],
) -> SideLayout {
    let granted = placed
        .iter()
        .map(|(kind, side)| {
            let x = match side {
                PanelSide::Left => frame.min.x - panel_size.x,
                PanelSide::Right => frame.max.x,
            };
            let min = Vec2::new(x, frame.min.y);
            (*kind, Rect::from_corners(min, min + panel_size))
        })
        .collect();
    let left = (frame.min.x > 0.0)
        .then(|| Rect::from_corners(Vec2::ZERO, Vec2::new(frame.min.x, window.y)));
    let right =
        (frame.max.x < window.x).then(|| Rect::from_corners(Vec2::new(frame.max.x, 0.0), window));
    SideLayout {
        left,
        right,
        frame: Some(frame),
        granted,
    }
}

/// The run condition on the faithful `fit_display_to_window`, in both
/// update chains: true when there is no [`SideLayout`] (every Classic run)
/// or it does not own the frame.
pub fn faithful_fit_owns_frame(layout: Option<Res<SideLayout>>) -> bool {
    layout.is_none_or(|layout| !layout.owns_frame())
}

/// Whether the toggles in force turn on a side-claiming feature.
pub fn side_claim_on(toggles: FeatureToggles) -> bool {
    SIDE_CLAIM_KEYS
        .iter()
        .filter_map(|key| Toggle::from_key(key))
        .any(|toggle| toggles.on(toggle))
}

/// Adds the arbiter when a side-claiming feature is on; otherwise adds
/// nothing. Called from [`super::EnhancementsPlugin`].
pub(super) fn add_side_space(app: &mut App, toggles: FeatureToggles, values: &SettingValues) {
    if side_claim_on(toggles) {
        install_side_space(app, SideSpaceSettings::from_values(values));
    }
}

/// The registry, the layout, the settings and the three systems, once.
/// A panel that finds no room says so with a toast, so the shell services
/// come with it.
pub fn install_side_space(app: &mut App, settings: SideSpaceSettings) {
    if app.world().contains_resource::<PanelRegistry>() {
        return;
    }
    install_shell_services(app);
    app.init_resource::<PanelRegistry>()
        .init_resource::<SideLayout>()
        .init_resource::<HiddenPanelsToasted>()
        .insert_resource(settings)
        .add_systems(
            Update,
            (layout_side_space, fit_frame_to_side_layout)
                .chain()
                .run_if(side_space_active)
                .in_set(EnhSet::Layout),
        )
        .add_systems(Update, draw_side_panels.in_set(EnhSet::Present));
}

/// A claim to place, a granted layout to release, or a scale policy.
fn side_space_active(
    registry: Res<PanelRegistry>,
    layout: Res<SideLayout>,
    policy: Option<Res<FramePolicy>>,
) -> bool {
    !registry.is_empty() || layout.owns_frame() || policy.is_some()
}

/// The kinds already told they found no room, so the toast shows once a
/// run.
#[derive(Resource, Debug, Default)]
pub(crate) struct HiddenPanelsToasted(Vec<PanelKind>);

/// Runs [`solve_frame`] for the window, the registry's claims and the scale
/// policy, in `EnhSet::Layout`, and stores the result when it changed.
#[allow(clippy::too_many_arguments)]
pub(crate) fn layout_side_space(
    windows: Query<&Window>,
    registry: Res<PanelRegistry>,
    settings: Res<SideSpaceSettings>,
    policy: Option<Res<FramePolicy>>,
    image: Option<Res<FrameImageSize>>,
    mut layout: ResMut<SideLayout>,
    mut toasted: ResMut<HiddenPanelsToasted>,
    toasts: Option<ResMut<Toasts>>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let solution = solve_frame(
        WindowPixels::of(window),
        FrameImageSize::image(image.as_deref()),
        &registry.claims(),
        &settings,
        policy.as_deref().map(|policy| policy.0.as_ref()),
    );
    if *layout != solution.layout {
        *layout = solution.layout;
    }
    let fresh: Vec<PanelKind> = solution
        .hidden
        .into_iter()
        .filter(|kind| !toasted.0.contains(kind))
        .collect();
    if fresh.is_empty() {
        return;
    }
    toasted.0.extend(fresh);
    if let Some(mut toasts) = toasts {
        toasts.short(PANEL_HIDDEN);
    }
}

/// Sizes and places every `Sprite`, as `fit_display_to_window` does, while
/// the arbiter owns the frame; on the frame it stops owning it, puts them
/// back where the faithful fit keeps them (its size, centred).
fn fit_frame_to_side_layout(
    windows: Query<&Window>,
    layout: Res<SideLayout>,
    mut owned: Local<bool>,
    mut sprites: Query<(&mut Sprite, &mut Transform)>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let size = Vec2::new(window.resolution.width(), window.resolution.height());
    let (frame_size, centre) = match layout.frame {
        Some(frame) => (
            frame.size(),
            Vec2::new(
                frame.center().x - size.x / 2.0,
                size.y / 2.0 - frame.center().y,
            ),
        ),
        None if *owned => (
            fitted_display_size(VISUAL_PLAY_FRAME_WIDTH, VISUAL_PLAY_FRAME_HEIGHT, size),
            Vec2::ZERO,
        ),
        None => return,
    };
    *owned = layout.owns_frame();
    for (mut sprite, mut transform) in &mut sprites {
        if sprite.custom_size != Some(frame_size) {
            sprite.custom_size = Some(frame_size);
        }
        if transform.translation.truncate() != centre {
            transform.translation.x = centre.x;
            transform.translation.y = centre.y;
        }
    }
}

/// A granted panel's node and what its image shows.
#[derive(Component, Debug)]
pub struct SidePanelNode {
    kind: PanelKind,
    side: PanelSide,
    drawn: PanelContent,
}

/// The cell panel's pixels: the gutter on the frame side, then the title
/// and rows in `font`, one cell per byte. A byte the font has no glyph for
/// draws blank.
pub fn render_cell_panel(font: &FixedCellFont, rows: &PanelRows, side: PanelSide) -> Vec<u8> {
    let width = PANEL_WIDTH as usize;
    let mut rgba = PAPER.repeat(width * PANEL_HEIGHT as usize);
    let text_left = match side {
        PanelSide::Left => 0,
        PanelSide::Right => PANEL_GUTTER as usize,
    };
    for (row, cells) in rows.rows.iter().enumerate() {
        let ink = if row == 0 { TITLE_INK } else { ROW_INK };
        for (cell, byte) in cells.iter().enumerate() {
            for line in 0..CH_CELL_SIDE {
                let bits = font.glyph_row(*byte, line).unwrap_or(0);
                for column in 0..CH_CELL_SIDE {
                    if bits & (0x80 >> column) == 0 {
                        continue;
                    }
                    let x = text_left + cell * CH_CELL_SIDE + column;
                    let y = row * CH_CELL_SIDE + line;
                    let offset = (y * width + x) * 4;
                    rgba[offset..offset + 4].copy_from_slice(&ink);
                }
            }
        }
    }
    rgba
}

fn panel_image(rgba: Vec<u8>) -> Image {
    let mut image = Image::new(
        Extent3d {
            width: PANEL_WIDTH,
            height: PANEL_HEIGHT,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        rgba,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::nearest();
    image
}

fn place(node: &mut Node, rect: Rect) {
    let (left, top) = (Val::Px(rect.min.x), Val::Px(rect.min.y));
    let (width, height) = (Val::Px(rect.width()), Val::Px(rect.height()));
    if node.position_type != PositionType::Absolute {
        node.position_type = PositionType::Absolute;
    }
    if (node.left, node.top, node.width, node.height) != (left, top, width, height) {
        node.left = left;
        node.top = top;
        node.width = width;
        node.height = height;
    }
}

/// The panel host, in `EnhSet::Present`: one `bevy_ui` node per granted
/// panel showing its provider's snapshot. A cell panel's image is rebuilt
/// only when its cells or side change. A panel no longer granted, or whose
/// provider shows nothing, has no node.
fn draw_side_panels(
    mut commands: Commands,
    layout: Res<SideLayout>,
    registry: Res<PanelRegistry>,
    visual: Option<Res<VisualState>>,
    mut images: ResMut<Assets<Image>>,
    mut nodes: Query<(Entity, &mut SidePanelNode, &mut Node, &mut ImageNode)>,
) {
    let state = visual.as_deref().map(|visual| &visual.state);
    let font = visual.as_deref().map(|visual| &visual.text_font);
    let mut wanted: Vec<(PanelKind, PanelSide, Rect, PanelContent)> = Vec::new();
    for (kind, rect) in &layout.granted {
        let (Some(provider), Some(side)) = (registry.get(*kind), layout.side_of(*kind)) else {
            continue;
        };
        let Some(content) = provider.snapshot(state) else {
            continue;
        };
        if matches!(content, PanelContent::Cells(_)) && font.is_none() {
            continue;
        }
        wanted.push((*kind, side, *rect, content));
    }

    for (entity, mut panel, mut node, mut image_node) in &mut nodes {
        let Some(index) = wanted.iter().position(|(kind, ..)| *kind == panel.kind) else {
            commands.entity(entity).despawn();
            continue;
        };
        let (_, side, rect, content) = wanted.swap_remove(index);
        place(&mut node, rect);
        if panel.side == side && panel.drawn == content {
            continue;
        }
        match (&content, font) {
            (PanelContent::Cells(rows), Some(font)) => {
                let rgba = render_cell_panel(font, rows, side);
                // Our own image is repainted in place; a provider's image
                // (the node showed one before) is never written.
                let own = !matches!(panel.drawn, PanelContent::Image(_));
                match images.get_mut(&image_node.image).filter(|_| own) {
                    Some(image) => image.data = Some(rgba),
                    None => image_node.image = images.add(panel_image(rgba)),
                }
            }
            (PanelContent::Image(handle), _) => image_node.image = handle.clone(),
            (PanelContent::Cells(_), None) => unreachable!("filtered above"),
        }
        panel.side = side;
        panel.drawn = content;
    }

    for (kind, side, rect, content) in wanted {
        let image = match (&content, font) {
            (PanelContent::Cells(rows), Some(font)) => {
                images.add(panel_image(render_cell_panel(font, rows, side)))
            }
            (PanelContent::Image(handle), _) => handle.clone(),
            (PanelContent::Cells(_), None) => continue,
        };
        let mut node = Node::default();
        place(&mut node, rect);
        commands.spawn((
            Name::new("enh side panel"),
            SidePanelNode {
                kind,
                side,
                drawn: content,
            },
            node,
            ImageNode::new(image),
        ));
    }
}

#[cfg(test)]
pub(crate) mod tests;

//! The enhanced gem map on screen (Phase B design 10.3.5, 10.2.10,
//! 10.2.11; build plan M3-11, D-M3.4).
//!
//! While the game's own gem view is up, the gem map covers the whole window
//! with one `ImageNode` ([`GemMapNode`]) at [`GEM_NODE_Z`]: above the frame
//! sprite, the side fill (`GlobalZIndex(-1)`) and the side panels, below
//! the toasts (`i32::MAX - 1`). It is a UI node, never a sprite, so the
//! faithful `fit_display_to_window` is not touched, and the faithful frame
//! under it is drawn and hashed exactly as before.
//!
//! - `gem_capture_pre` (`EnhSet::Feed`, after the probe) copies the frame
//!   sprite's current picture into the [`GemBefore`] image, only while
//!   `gem_map.sweep` is on and a key is queued or pressed this frame with no
//!   gem view up: the view can only open on such a frame, and by
//!   `EnhSet::Present` the frame already shows the 1.0.1 gem view (D-M3.4).
//! - `gem_render` (`EnhSet::Present`) runs only while a gem view is up or
//!   the map is still showing. It renders the map ([`render_gem_map`]) only
//!   when a view opens, its page changes or the canvas changes with the
//!   window, and recomposes the node's picture only when the sweep step
//!   changes.
//! - `gem_layout` (`EnhSet::Present`, after the render) spawns, sizes and
//!   removes the nodes. The [`GemBeforeNode`] sits directly under the map,
//!   over the frame sprite's rectangle, only while an open sweep runs.
//! - `gem_hide` runs only when the toggle is off and a node is left: the
//!   map goes on the next frame and the game's own view shows.
//!
//! **The sweep** (10.2.10, D-M3.4) is chapter 2's rising dither,
//! [`effects::sweep_t`] over the whole canvas (`top = 0`, `bottom = h`),
//! in 8 steps of [`GEM_SWEEP_STEP`]: at step `k` of 1..=7 a pixel shows the
//! map where its step is `k` or less, and is transparent elsewhere, so the
//! "before" picture (opening) or the live frame (closing) shows through.
//! A key that closes the view mid-sweep reverses it from the step it is on.
//! The sweep is drawn only: it holds no key and feeds nothing, and the
//! game closes its view on the very frame it would without the feature.
//!
//! The canvas is the foundation's for the window ([`Canvas::for_window`]),
//! or the classic 320x200; below a 320x200 window there is no gem map and
//! the 1.0.1 view shows (D-M3.3).
//!
//! **Dungeons** (10.2.6, M3-13): the level stack, drawn with the IBM.CH and
//! RUNES.CH fonts the shell already holds, the record and
//! `gem_map.glyphs`.
//!
//! **Towns** (10.2.5, M3-12): the land beside the floor comes from the
//! static world map of the town's plane, read through [`StaticMaps`] from
//! the folder the game plays from on the first town gem with
//! `gem_map.town_surroundings` on, and kept. Only the cells the record says
//! the party has seen are drawn. A map that cannot be read leaves the sides
//! black.

use std::time::Duration;

use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use u5_automap::StaticMaps;
use u5_automap::gem::{
    DungeonInputs, GemCanvas, GemImage, GemRender, GemShown, GemViewState, GemWindow, MapArt,
    TownInputs, gem_canvas, gem_view_open, render_gem_map, town_world_plane,
};
use u5_runtime::enhancements::{GemMapToggles, PixelAspect, ResolvedEnhancements};
use u5_runtime::{Area, PlayState};

use super::{GEM, GemMap};
use crate::VisualState;
use crate::display::effects::{STEP_SLACK, SWEEP_STEPS, step_of, sweep_t};
use crate::display::grow::{CLASSIC_BELOW, Canvas};
use crate::enh::{EnhLaunch, Toggles};

/// The map node's place: above everything the shell draws, under the
/// toasts (`GlobalZIndex(i32::MAX - 1)`).
pub const GEM_NODE_Z: i32 = i32::MAX - 2;

/// The "before" node, directly under the map.
pub const GEM_BEFORE_Z: i32 = i32::MAX - 3;

/// One sweep step (10.2.10): 8 steps of about 60 ms.
pub const GEM_SWEEP_STEP: Duration = Duration::from_millis(60);

/// The full-window gem map.
#[derive(Component, Debug)]
pub struct GemMapNode;

/// The sweep's "before" picture over the frame sprite (D-M3.4).
#[derive(Component, Debug)]
pub struct GemBeforeNode;

/// The sweep's state: where it is going and the step it shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GemSweep {
    /// True while the map is (or is becoming) shown.
    pub open: bool,
    /// The step drawn, 1..=7, while a sweep runs; `None` when it is over:
    /// the whole map while `open`, nothing otherwise.
    pub step: Option<u8>,
    elapsed: Duration,
}

impl GemSweep {
    /// A view opened: sweep in from step 1, or reverse a closing sweep from
    /// its step. Without the sweep the map shows at once.
    pub fn start_open(&mut self, sweep: bool) {
        self.open = true;
        self.step = sweep.then(|| self.step.unwrap_or(1));
        self.elapsed = Duration::ZERO;
    }

    /// The view closed: sweep out from step 7, or reverse an opening sweep
    /// from its step. Without the sweep the map goes at once.
    pub fn start_close(&mut self, sweep: bool) {
        self.open = false;
        self.step = sweep.then(|| self.step.unwrap_or(SWEEP_STEPS - 1));
        self.elapsed = Duration::ZERO;
    }

    /// Gone at once (no canvas, the toggle off, an undrawn view).
    pub fn stop(&mut self) {
        *self = Self::default();
    }

    /// Whether anything of the map shows.
    pub fn visible(&self) -> bool {
        self.open || self.step.is_some()
    }

    /// `dt` of shell frame time passed: a step every [`GEM_SWEEP_STEP`],
    /// towards 8 (all map) when opening and 0 (none) when closing. With the
    /// sweep turned off it ends where it is going.
    pub fn tick(&mut self, dt: Duration, sweep: bool) {
        if self.step.is_none() || !sweep {
            self.step = None;
            self.elapsed = Duration::ZERO;
            return;
        }
        self.elapsed += dt;
        while let Some(k) = self.step
            && self.elapsed + STEP_SLACK >= GEM_SWEEP_STEP
        {
            self.elapsed = self.elapsed.saturating_sub(GEM_SWEEP_STEP);
            let next = if self.open {
                k + 1
            } else {
                k.saturating_sub(1)
            };
            self.step = (1..SWEEP_STEPS).contains(&next).then_some(next);
        }
        if self.step.is_none() {
            self.elapsed = Duration::ZERO;
        }
    }
}

/// Each canvas pixel's sweep step (1..=8): the first step that shows the
/// map there, `step_of(sweep_t(x, y, 0, h))` (10.2.10, D-M3.4).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SweepSteps {
    pub canvas: GemCanvas,
    steps: Vec<u8>,
}

impl SweepSteps {
    pub fn new(canvas: GemCanvas) -> Self {
        let mut steps = Vec::with_capacity(canvas.pixels());
        for y in 0..canvas.h {
            for x in 0..canvas.w {
                steps.push(step_of(sweep_t(x, y, 0, canvas.h)));
            }
        }
        Self { canvas, steps }
    }

    /// The step from which pixel `(x, y)` shows the map.
    pub fn at(&self, x: u32, y: u32) -> u8 {
        self.steps[(y * self.canvas.w + x) as usize]
    }
}

/// The node's picture at sweep step `k` (`None`: the whole map): the map's
/// pixel where its step is `k` or less, transparent elsewhere.
pub fn sweep_picture(full: &GemImage, steps: &SweepSteps, k: Option<u8>, out: &mut Vec<u8>) {
    out.clear();
    out.extend_from_slice(&full.rgba);
    let Some(k) = k else {
        return;
    };
    for (pixel, &step) in out.as_chunks_mut::<4>().0.iter_mut().zip(&steps.steps) {
        if step > k {
            *pixel = [0; 4];
        }
    }
}

/// The gem map's canvas for a window of `physical` pixels (D-M3.3,
/// 10.2.4): the foundation's canvas on a window wider than 4:3 (512x240 at
/// 16:9, 688x240 at 21:9, 480x216 on 1080 lines), the original 320x200
/// screen on a 4:3 window or a narrower one, and none below 320x200.
///
/// At exactly 4:3 the foundation's canvas is 384x240 (`CLASSIC_BELOW`
/// wide), the same shape as 320x200; the approved 4:3 gem map (10.2.4,
/// mockup `O3`) is the original screen's 320x200, 80 x 50 cells, so a
/// canvas no wider than that takes the classic size.
pub fn window_gem_canvas(physical: UVec2, aspect: PixelAspect) -> Option<GemCanvas> {
    let foundation = Canvas::for_window(physical.x, physical.y, aspect)
        .filter(|canvas| canvas.w > CLASSIC_BELOW)
        .map(|canvas| (canvas.w, canvas.h));
    gem_canvas(foundation, (physical.x, physical.y))
}

/// The sweep's "before" picture (D-M3.4): the frame sprite's picture copied
/// on the frame a gem view may open.
#[derive(Clone, Debug, Default)]
pub struct GemBefore {
    pub image: Option<Handle<Image>>,
    /// A picture was taken for the next open and not yet used.
    pub fresh: bool,
}

/// What the gem map shows and how far its sweep is. Exists only while the
/// plugin is in the app.
/// The static maps a town view's sides read, kept for the session.
#[derive(Default)]
struct GemStatics(Option<StaticMaps>);

impl std::fmt::Debug for GemStatics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GemStatics")
            .field("cached", &self.0.as_ref().map(StaticMaps::cached_len))
            .finish()
    }
}

#[derive(Resource, Debug, Default)]
pub struct GemShow {
    art: Option<MapArt>,
    statics: GemStatics,
    /// The open view, from its open until it closes.
    view: Option<GemViewState>,
    /// The gem count the open view opened with: each open spends a gem, so
    /// a new count is a new open even with no frame between.
    opened_gems: Option<u8>,
    full: GemImage,
    steps: Option<SweepSteps>,
    pub sweep: GemSweep,
    /// The node's picture and what it was composed from.
    image: Option<Handle<Image>>,
    composed: Option<(GemCanvas, Option<u8>, u64)>,
    picture: Vec<u8>,
    pub before: GemBefore,
    /// The open sweep shows the before picture.
    show_before: bool,
    /// How many times the map was rendered, and the picture composed.
    pub renders: u64,
    pub composes: u64,
}

impl GemShow {
    /// Whether the map shows on this frame.
    pub fn visible(&self) -> bool {
        self.sweep.visible() && self.image.is_some()
    }

    /// The map's picture, when it shows.
    pub fn image(&self) -> Option<&Handle<Image>> {
        self.image.as_ref().filter(|_| self.sweep.visible())
    }

    /// The canvas of the last render.
    pub fn canvas(&self) -> Option<GemCanvas> {
        self.steps.as_ref().map(|steps| steps.canvas)
    }

    fn hide(&mut self) {
        self.view = None;
        self.opened_gems = None;
        self.sweep.stop();
        self.show_before = false;
        self.composed = None;
    }
}

/// The settings the map reads: the launch's `[gem_map]` rows with the main
/// toggle as it is now, the pixel aspect and `display.map_brightness`.
fn settings(resolved: &ResolvedEnhancements) -> (GemMapToggles, PixelAspect, u8) {
    let mut t = resolved.gem_map();
    t.enhanced = true;
    (
        t,
        resolved.display().pixel_aspect,
        resolved.display_map_brightness(),
    )
}

/// The window's gem canvas, or `None` with no window or one too small.
fn current_canvas(windows: &Query<&Window>, aspect: PixelAspect) -> Option<GemCanvas> {
    windows
        .single()
        .ok()
        .and_then(|window| window_gem_canvas(window.physical_size(), aspect))
}

/// The outdoor window the enhanced map draws on this frame, for the
/// record's side cells (10.3.3): `None` unless a gem view is up outdoors
/// and the window holds a gem canvas.
pub(super) fn drawn_window(
    state: &PlayState,
    windows: &Query<&Window>,
    launch: &EnhLaunch,
) -> Option<GemWindow> {
    if !gem_view_open(state) || !matches!(state.area, Area::World { .. }) {
        return None;
    }
    let (_, aspect, _) = settings(&launch.0.enhancements);
    let canvas = current_canvas(windows, aspect)?;
    Some(u5_automap::gem::OutdoorLayout::new(canvas).gem_window())
}

fn rgba_image(w: u32, h: u32, rgba: Vec<u8>) -> Image {
    let mut image = Image::new(
        Extent3d {
            width: w,
            height: h,
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

/// Writes `rgba` into the image `handle` names when it has the same size,
/// else into a new image; returns the handle in use.
fn upload(
    images: &mut Assets<Image>,
    handle: Option<Handle<Image>>,
    size: UVec2,
    rgba: &[u8],
) -> Handle<Image> {
    if let Some(handle) = handle {
        if let Some(image) = images.get_mut(&handle)
            && image.size() == size
        {
            image.data = Some(rgba.to_vec());
            return handle;
        }
        images.remove(&handle);
    }
    images.add(rgba_image(size.x, size.y, rgba.to_vec()))
}

/// Run condition: something for `gem_render` to do: a gem view is up, or
/// the map still shows.
pub(super) fn gem_render_due(visual: Option<Res<VisualState>>, show: Res<GemShow>) -> bool {
    visual.is_some_and(|visual| gem_view_open(&visual.state))
        || show.view.is_some()
        || show.sweep.visible()
}

/// Copies the frame sprite's picture for the open sweep (D-M3.4), only
/// while the sweep is on and a key may open a gem view this frame.
pub(super) fn gem_capture_pre(
    gem: Res<GemMap>,
    mut show: ResMut<GemShow>,
    visual: Res<VisualState>,
    launch: Res<EnhLaunch>,
    windows: Query<&Window>,
    sprites: Query<&Sprite>,
    mut images: ResMut<Assets<Image>>,
) {
    let (t, aspect, _) = settings(&launch.0.enhancements);
    if gem.commands == 0 || !t.opts.sweep || gem_view_open(&visual.state) {
        return;
    }
    if current_canvas(&windows, aspect).is_none() {
        return;
    }
    let Some(sprite) = sprites.iter().next() else {
        return;
    };
    let Some((size, bytes)) = images
        .get(&sprite.image)
        .and_then(|image| Some((image.size(), image.data.clone()?)))
    else {
        return;
    };
    let handle = show.before.image.take();
    show.before.image = Some(upload(&mut images, handle, size, &bytes));
    show.before.fresh = true;
}

/// Renders the gem map on open, page and canvas changes, steps the sweep
/// and composes the node's picture when its step changed.
#[allow(clippy::too_many_arguments)]
pub(super) fn gem_render(
    mut show: ResMut<GemShow>,
    gem: Res<GemMap>,
    visual: Res<VisualState>,
    launch: Res<EnhLaunch>,
    windows: Query<&Window>,
    time: Res<Time>,
    mut images: ResMut<Assets<Image>>,
) {
    let show = &mut *show;
    let state = &visual.state;
    let (t, aspect, display_map) = settings(&launch.0.enhancements);
    let canvas = current_canvas(&windows, aspect);
    let open = gem_view_open(state);

    let Some(canvas) = canvas else {
        // Below a 320x200 window: the 1.0.1 view shows.
        show.hide();
        return;
    };

    let mut started = false;
    if open {
        let new_open = show.view.is_none() || show.opened_gems != Some(state.gems);
        let new_canvas = show.view.as_ref().is_some_and(|view| view.canvas != canvas);
        if new_open || new_canvas {
            let art = show.art.get_or_insert_with(|| {
                MapArt::build(&visual.atlas)
                    .with_fonts(visual.text_font.clone(), visual.rune_font.clone())
            });
            let surroundings = t.opts.town_surroundings;
            let world = town_world_plane(state)
                .filter(|_| surroundings)
                .and_then(|plane| {
                    let statics = show
                        .statics
                        .0
                        .get_or_insert_with(|| StaticMaps::new(&visual.game_dir));
                    statics.set_game_dir(&visual.game_dir);
                    statics.world_map(plane).ok()
                });
            let town = TownInputs {
                know: Some(gem.knowledge()),
                world,
                surroundings,
            };
            let dungeon = DungeonInputs {
                know: Some(gem.knowledge()),
                glyphs: t.opts.glyphs,
            };
            let view = GemViewState::open_all(state, canvas, display_map, town, dungeon);
            let drawn = render_gem_map(state, gem.knowledge(), art, &t, &view, &mut show.full);
            show.renders += 1;
            if drawn == GemRender::Cleared || matches!(view.shown, GemShown::Empty) {
                // A view this build does not draw: the 1.0.1 view shows.
                show.hide();
                return;
            }
            if show.canvas() != Some(canvas) {
                show.steps = Some(SweepSteps::new(canvas));
            }
            show.view = Some(view);
            show.composed = None;
            if new_open {
                show.opened_gems = Some(state.gems);
                show.show_before = t.opts.sweep && std::mem::take(&mut show.before.fresh);
                show.sweep.start_open(t.opts.sweep);
                started = true;
            }
        }
    } else if show.view.take().is_some() {
        show.opened_gems = None;
        show.show_before = false;
        show.sweep.start_close(t.opts.sweep);
        started = true;
    }
    if !started {
        show.sweep.tick(time.delta(), t.opts.sweep);
    }
    if !show.sweep.open && show.sweep.step.is_none() {
        show.show_before = false;
    }
    if !show.sweep.visible() {
        return;
    }

    let Some(steps) = show.steps.as_ref() else {
        return;
    };
    let key = (steps.canvas, show.sweep.step, show.renders);
    if show.composed == Some(key) && show.image.is_some() {
        return;
    }
    sweep_picture(&show.full, steps, show.sweep.step, &mut show.picture);
    let size = UVec2::new(steps.canvas.w, steps.canvas.h);
    let handle = show.image.take();
    show.image = Some(upload(&mut images, handle, size, &show.picture));
    show.composed = Some(key);
    show.composes += 1;
}

/// Run condition: the map shows, or one of its nodes is left to remove.
pub(super) fn gem_layout_due(
    show: Res<GemShow>,
    nodes: Query<(), Or<(With<GemMapNode>, With<GemBeforeNode>)>>,
) -> bool {
    show.visible() || !nodes.is_empty()
}

fn full_window() -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: Val::Px(0.0),
        top: Val::Px(0.0),
        width: Val::Percent(100.0),
        height: Val::Percent(100.0),
        ..default()
    }
}

/// The frame sprite's rectangle in window coordinates (logical pixels):
/// the 2D camera puts the world origin at the window centre, y up.
fn sprite_rect(window: &Window, sprite: &Sprite, transform: &Transform) -> Option<Rect> {
    let size = sprite.custom_size? * transform.scale.truncate();
    let centre = Vec2::new(
        window.resolution.width() / 2.0 + transform.translation.x,
        window.resolution.height() / 2.0 - transform.translation.y,
    );
    Some(Rect::from_center_size(centre, size))
}

/// Spawns, places and removes the map node and the before node.
pub(super) fn gem_layout(
    mut commands: Commands,
    mut show: ResMut<GemShow>,
    windows: Query<&Window>,
    sprites: Query<(&Sprite, &Transform)>,
    mut maps: Query<(Entity, &mut ImageNode), (With<GemMapNode>, Without<GemBeforeNode>)>,
    mut befores: Query<(Entity, &mut Node, &mut ImageNode), With<GemBeforeNode>>,
) {
    // The map.
    match show.image().cloned() {
        Some(image) => {
            let mut placed = false;
            for (entity, mut node) in &mut maps {
                if placed {
                    commands.entity(entity).despawn();
                    continue;
                }
                if node.image != image {
                    node.image = image.clone();
                }
                placed = true;
            }
            if !placed {
                commands.spawn((
                    Name::new("enh gem map"),
                    GemMapNode,
                    full_window(),
                    ImageNode::new(image),
                    GlobalZIndex(GEM_NODE_Z),
                ));
            }
        }
        None => {
            for (entity, _) in &maps {
                commands.entity(entity).despawn();
            }
        }
    }

    // The before picture, over the frame sprite, while an open sweep runs.
    let rect = windows.single().ok().and_then(|window| {
        let (sprite, transform) = sprites.iter().next()?;
        sprite_rect(window, sprite, transform)
    });
    let before = show
        .before
        .image
        .clone()
        .filter(|_| show.show_before && show.sweep.open && show.sweep.step.is_some());
    match (before, rect) {
        (Some(image), Some(rect)) => {
            let mut placed = false;
            for (entity, mut node, mut image_node) in &mut befores {
                if placed {
                    commands.entity(entity).despawn();
                    continue;
                }
                place(&mut node, rect);
                if image_node.image != image {
                    image_node.image = image.clone();
                }
                placed = true;
            }
            if !placed {
                let mut node = full_window();
                place(&mut node, rect);
                commands.spawn((
                    Name::new("enh gem before"),
                    GemBeforeNode,
                    node,
                    ImageNode::new(image),
                    GlobalZIndex(GEM_BEFORE_Z),
                ));
            }
        }
        _ => {
            for (entity, ..) in &befores {
                commands.entity(entity).despawn();
            }
            if !show.sweep.open || show.sweep.step.is_none() {
                show.show_before = false;
            }
        }
    }
}

fn place(node: &mut Node, rect: Rect) {
    let (left, top) = (Val::Px(rect.min.x), Val::Px(rect.min.y));
    let (width, height) = (Val::Px(rect.width()), Val::Px(rect.height()));
    if node.left != left {
        node.left = left;
    }
    if node.top != top {
        node.top = top;
    }
    if node.width != width {
        node.width = width;
    }
    if node.height != height {
        node.height = height;
    }
}

/// Run condition: the toggle is off and something of the map is left.
pub(super) fn gem_hide_due(
    toggles: Option<Res<Toggles>>,
    show: Res<GemShow>,
    nodes: Query<(), Or<(With<GemMapNode>, With<GemBeforeNode>)>>,
) -> bool {
    !toggles.is_some_and(|t| t.0.on(GEM)) && (show.sweep.visible() || !nodes.is_empty())
}

/// The toggle went off with the map up: it goes on this frame, and the
/// game's own view shows (10.2.11).
pub(super) fn gem_hide(
    mut commands: Commands,
    mut show: ResMut<GemShow>,
    nodes: Query<Entity, Or<(With<GemMapNode>, With<GemBeforeNode>)>>,
) {
    show.hide();
    for entity in &nodes {
        commands.entity(entity).despawn();
    }
}

#[cfg(test)]
#[path = "view_tests.rs"]
mod tests;

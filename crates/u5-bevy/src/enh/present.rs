//! The presentation pipeline host (Phase B design 1.3.8, U10).
//!
//! Every stage that draws on top of the finished original frame is a
//! [`PresentStep`] of this one pipeline. The faithful image (320x200,
//! written by the existing upload sites) is never written: the host copies
//! it, runs the registered steps on the copy in [`StepOrder`], and shows
//! the copy on the frame sprite.
//!
//! ```text
//! faithful Image  --(hash, animation, flash, layout or a step revision)-->
//!   [composite] [highlight] [grow-left compose] [brightness] [prescale]
//!   --> presented Image --> frame sprite
//! ```
//!
//! - [`PresentSteps`] holds the steps; [`pipeline_active`] is true while it
//!   holds any. `present_frame` runs in [`EnhSet::Present`], after the camp
//!   apparition flash and before screenshots.
//! - The host keeps the 64-bit FNV-1a hash of the faithful bytes and
//!   recomposes only when that hash, the animation phase, a flash, the
//!   layout (window or side layout) or a step's [`PresentStep::revision`]
//!   changed, or the step list did. A new highlight set therefore repaints
//!   even when the faithful image is unchanged.
//! - One presented image is kept per geometry (size and sampler) and
//!   repainted in place until the geometry changes, when it is replaced.
//! - No upload site is edited; detecting the change downstream means a
//!   future upload site cannot be missed.
//!
//! Nothing here is in the app until a feature registers a step
//! ([`register_present_step`]): with no step the frame sprite shows the
//! faithful handle, exactly as in 1.0.1, and no host system exists. When
//! the last step is removed in play, the host hands the sprite back to the
//! faithful handle and drops the presented image.

use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use u5_runtime::{FixedCellFont, PlayState, TileAtlas, hash_bytes};

use super::EnhSet;
use super::side::SideLayout;
use crate::{VisualIntroState, VisualState};

/// A step's place in the pipeline: lower runs first. The design's steps
/// have fixed places (1.3.8, 9.3.10); a step between two of them picks a
/// value between theirs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StepOrder(pub u16);

impl StepOrder {
    /// Chapter 9's sprite compositing: first, so it sees the faithful pixels.
    pub const COMPOSITE: StepOrder = StepOrder(100);
    /// Chapter 8's clue highlighting: before the grow-left compositor copies
    /// the core.
    pub const HIGHLIGHT: StepOrder = StepOrder(200);
    /// Chapter 2's grow-left canvas (T2).
    pub const GROW_LEFT_COMPOSE: StepOrder = StepOrder(300);
    /// Chapter 9's brightness and vignette: after the canvas, before the
    /// prescale.
    pub const BRIGHTNESS: StepOrder = StepOrder(400);
    /// Chapter 2's sharp-bilinear prescale (T0): last.
    pub const PRESCALE: StepOrder = StepOrder(500);

    /// The design's steps, in pipeline order.
    pub const ALL: [StepOrder; 5] = [
        StepOrder::COMPOSITE,
        StepOrder::HIGHLIGHT,
        StepOrder::GROW_LEFT_COMPOSE,
        StepOrder::BRIGHTNESS,
        StepOrder::PRESCALE,
    ];
}

/// What a step is shown: the faithful frame and what it was drawn from.
/// Everything is shared; a step cannot write the game.
pub struct PresentInput<'a> {
    /// The faithful image's RGBA bytes.
    pub faithful: &'a [u8],
    /// Its size in pixels.
    pub size: UVec2,
    /// The 64-bit FNV-1a hash of `faithful`.
    pub faithful_hash: u64,
    /// The game, once play has started (`None` in the intro).
    pub state: Option<&'a PlayState>,
    /// The tile atlas the faithful frame was drawn with, once play has
    /// started (the grow-left compositor paints the ring from it).
    pub atlas: Option<&'a TileAtlas>,
    /// The runtime-loaded IBM.CH the faithful frame's text is drawn with,
    /// once play has started (the grow-left ribbon draws from it).
    pub font: Option<&'a FixedCellFont>,
    /// The window's physical size (zero when there is no window).
    pub window: UVec2,
    /// Physical pixels per logical pixel (1 when there is no window), so a
    /// step can turn the side layout's logical rects into physical pixels.
    pub scale_factor: f32,
    /// The side-space arbiter's layout, while the arbiter is in the app.
    pub side: Option<&'a SideLayout>,
    /// The faithful frame is the last frame of a visibility sweep (the
    /// White potion): the state has already left the sweep, but its
    /// top-down visibility buffers predate the sweep until the idle redraw
    /// that follows on the next tick. A step that reads those buffers (the
    /// object-cell peek, design 9.3.7) passes this frame through.
    pub sweep_settling: bool,
}

/// The presented copy a step draws on. It starts as the faithful frame; a
/// step may change its size (the grow-left canvas, the prescale) or ask
/// for a smoothing sampler (the prescale).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PresentedFrame {
    pub size: UVec2,
    pub rgba: Vec<u8>,
    /// Shown with a linear sampler instead of the faithful nearest one.
    pub smooth: bool,
    /// Where the map is on the copy, when a step that moved it said so
    /// (the grow-left canvas, build plan D-M2.5). The host clears it before
    /// the steps run; without it a later step takes the 320x200 viewport.
    pub map_region: Option<MapRegion>,
}

/// The map on a presented copy (design 9.3.3, build plan D-M2.5): the map
/// rectangles, less the pixels inside them that are text drawn over the
/// map (the grow-left crawl's glyphs, outlines and end cap). A step that
/// treats the map alone (the brightness step's scope `map`) reads it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MapRegion {
    /// The copy's size the region was made for.
    pub size: UVec2,
    /// The map rectangles, `(x0, y0, x1, y1)` inclusive, not overlapping.
    pub rects: Vec<(u32, u32, u32, u32)>,
    /// One flag per pixel of `size`, set where the pixel is not map; empty
    /// while no pixel is excluded.
    excluded: Vec<bool>,
}

impl MapRegion {
    /// The rectangle `rect` of a `size` copy.
    pub fn rect(size: UVec2, rect: (u32, u32, u32, u32)) -> Self {
        Self {
            size,
            rects: vec![rect],
            excluded: Vec::new(),
        }
    }

    /// Leave pixel `(x, y)` out (outside the copy: ignored).
    pub fn exclude(&mut self, x: u32, y: u32) {
        if x >= self.size.x || y >= self.size.y {
            return;
        }
        if self.excluded.is_empty() {
            self.excluded = vec![false; (self.size.x * self.size.y) as usize];
        }
        self.excluded[(y * self.size.x + x) as usize] = true;
    }

    /// Whether `(x, y)` was left out.
    pub fn excludes(&self, x: u32, y: u32) -> bool {
        !self.excluded.is_empty()
            && x < self.size.x
            && y < self.size.y
            && self.excluded[(y * self.size.x + x) as usize]
    }

    /// Whether `(x, y)` is map: in a rectangle and not left out.
    pub fn contains(&self, x: u32, y: u32) -> bool {
        self.rects
            .iter()
            .any(|&(x0, y0, x1, y1)| (x0..=x1).contains(&x) && (y0..=y1).contains(&y))
            && !self.excludes(x, y)
    }

    /// How many pixels of the copy are map.
    pub fn count(&self) -> usize {
        let mut count = 0;
        self.for_each_run(|_, x0, x1| count += (x1 - x0) as usize);
        count
    }

    /// Calls `f` with each run of map pixels, `(y, x0, x1)` with `x1`
    /// exclusive, row by row, clipped to the copy.
    pub fn for_each_run(&self, mut f: impl FnMut(u32, u32, u32)) {
        let (w, h) = (self.size.x, self.size.y);
        for &(x0, y0, x1, y1) in &self.rects {
            if x0 >= w || y0 >= h || x0 > x1 || y0 > y1 {
                continue;
            }
            let (x1, y1) = (x1.min(w - 1), y1.min(h - 1));
            for y in y0..=y1 {
                if self.excluded.is_empty() {
                    f(y, x0, x1 + 1);
                    continue;
                }
                let row = &self.excluded[(y * w) as usize..((y + 1) * w) as usize];
                let mut x = x0;
                while x <= x1 {
                    if row[x as usize] {
                        x += 1;
                        continue;
                    }
                    let start = x;
                    while x <= x1 && !row[x as usize] {
                        x += 1;
                    }
                    f(y, start, x);
                }
            }
        }
    }
}

impl PresentedFrame {
    /// Resizes to `size`, black and opaque. A map region set before no
    /// longer fits and is dropped.
    pub fn resize(&mut self, size: UVec2) {
        self.map_region = None;
        self.size = size;
        self.rgba.clear();
        self.rgba
            .extend([0x00, 0x00, 0x00, 0xff].repeat((size.x * size.y) as usize));
    }

    fn geometry(&self) -> (UVec2, bool) {
        (self.size, self.smooth)
    }
}

/// One stage of the pipeline. Features register steps in [`PresentSteps`]
/// while their toggle is on.
pub trait PresentStep: Send + Sync + 'static {
    /// Where it runs; one step per order.
    fn order(&self) -> StepOrder;

    /// Changes whenever the step would draw something different from the
    /// same faithful frame (a new highlight set, the water phase under a
    /// ship). Asked every frame the host runs, so it should be cheap.
    fn revision(&mut self, input: &PresentInput) -> u64;

    /// Draws on the presented copy. Never sees the faithful image as
    /// writable.
    fn apply(&mut self, input: &PresentInput, frame: &mut PresentedFrame);
}

/// The registered steps, kept in [`StepOrder`].
#[derive(Resource, Default)]
pub struct PresentSteps(Vec<Box<dyn PresentStep>>);

impl PresentSteps {
    /// Adds `step`, replacing one of the same order.
    pub fn register(&mut self, step: impl PresentStep) {
        self.remove(step.order());
        self.0.push(Box::new(step));
        self.0.sort_by_key(|step| step.order());
    }

    /// Removes the step at `order`, when its feature goes off.
    pub fn remove(&mut self, order: StepOrder) {
        self.0.retain(|step| step.order() != order);
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The registered orders, first to last.
    pub fn orders(&self) -> Vec<StepOrder> {
        self.0.iter().map(|step| step.order()).collect()
    }
}

/// Run condition: some step is registered. False when the pipeline is not
/// in the app at all.
pub fn pipeline_active(steps: Option<Res<PresentSteps>>) -> bool {
    steps.is_some_and(|steps| !steps.is_empty())
}

/// Adds the host the first time a step is registered, then the step.
pub fn register_present_step(app: &mut App, step: impl PresentStep) {
    install_present_pipeline(app);
    app.world_mut()
        .resource_mut::<PresentSteps>()
        .register(step);
}

/// The step list, the host's state and `present_frame`, once.
pub fn install_present_pipeline(app: &mut App) {
    if app.world().contains_resource::<PresentSteps>() {
        return;
    }
    app.init_resource::<PresentSteps>()
        .init_resource::<PresentHost>()
        .add_systems(
            Update,
            present_frame.run_if(host_has_work).in_set(EnhSet::Present),
        );
}

/// A step to run, or a presented image to hand back.
fn host_has_work(steps: Res<PresentSteps>, host: Res<PresentHost>) -> bool {
    !steps.is_empty() || host.presented.is_some()
}

/// Everything a recompose depends on. The host recomposes when it differs
/// from the last one.
#[derive(Clone, Debug, PartialEq)]
struct Triggers {
    faithful: AssetId<Image>,
    faithful_hash: u64,
    size: UVec2,
    /// `PlayState::animation`'s shared phase.
    animation: Option<u8>,
    /// The camp and potion flash and the roster row flash.
    flash: Option<(bool, Option<usize>)>,
    window: UVec2,
    side_frame: Option<Rect>,
    /// Each step's order and revision, first to last.
    steps: Vec<(StepOrder, u64)>,
}

/// The host's own state: the last triggers, the presented image and its
/// geometry, and the working copies it reuses.
#[derive(Resource, Default)]
pub struct PresentHost {
    last: Option<Triggers>,
    presented: Option<Handle<Image>>,
    geometry: (UVec2, bool),
    source: Vec<u8>,
    frame: PresentedFrame,
    composes: u64,
}

impl PresentHost {
    /// The image the frame sprite shows while a step is registered.
    pub fn presented(&self) -> Option<&Handle<Image>> {
        self.presented.as_ref()
    }

    /// How many times the pipeline has recomposed.
    pub fn composes(&self) -> u64 {
        self.composes
    }
}

fn presented_image(frame: &PresentedFrame) -> Image {
    let mut image = Image::new(
        Extent3d {
            width: frame.size.x,
            height: frame.size.y,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        frame.rgba.clone(),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = if frame.smooth {
        ImageSampler::linear()
    } else {
        ImageSampler::nearest()
    };
    image
}

/// Points every sprite showing `from` at `to`. The shells have one sprite,
/// the frame.
fn retarget_sprites(sprites: &mut Query<&mut Sprite>, from: &Handle<Image>, to: &Handle<Image>) {
    for mut sprite in sprites.iter_mut() {
        if sprite.image == *from {
            sprite.image = to.clone();
        }
    }
}

/// The host, in `EnhSet::Present`. Reads the faithful image of whichever
/// shell is showing (play or intro), recomposes on a trigger, and shows
/// the presented copy on the frame sprite. With no step left it shows the
/// faithful image again and drops the copy.
#[allow(clippy::too_many_arguments)]
pub(crate) fn present_frame(
    mut host: ResMut<PresentHost>,
    mut steps: ResMut<PresentSteps>,
    visual: Option<Res<VisualState>>,
    intro: Option<Res<VisualIntroState>>,
    side: Option<Res<SideLayout>>,
    windows: Query<&Window>,
    mut images: ResMut<Assets<Image>>,
    mut sprites: Query<&mut Sprite>,
) {
    let faithful = match (&visual, &intro) {
        (Some(visual), _) => Some(visual.image_handle.clone()),
        (None, Some(intro)) => intro.image_handle.clone(),
        (None, None) => None,
    };

    if steps.is_empty() {
        if let Some(presented) = host.presented.take() {
            if let Some(faithful) = &faithful {
                retarget_sprites(&mut sprites, &presented, faithful);
            }
            images.remove(&presented);
        }
        host.last = None;
        return;
    }
    let Some(faithful) = faithful else {
        return;
    };

    let host = &mut *host;
    let state = visual.as_deref().map(|visual| &visual.state);
    let atlas = visual.as_deref().map(|visual| &visual.atlas);
    let font = visual.as_deref().map(|visual| &visual.text_font);
    let (window, scale_factor) = windows
        .single()
        .map(|window| (window.physical_size(), window.scale_factor()))
        .unwrap_or((UVec2::ZERO, 1.0));
    let side = side.as_deref();
    let sweep_settling = visual
        .as_deref()
        .is_some_and(|visual| visual.sweep_idle_redraw_pending);
    let triggers = {
        let Some(image) = images.get(&faithful) else {
            return;
        };
        let Some(bytes) = image.data.as_deref() else {
            return;
        };
        let size = image.size();
        let faithful_hash = hash_bytes(bytes);
        let input = PresentInput {
            faithful: bytes,
            size,
            faithful_hash,
            state,
            atlas,
            font,
            window,
            scale_factor,
            side,
            sweep_settling,
        };
        let revisions = steps
            .0
            .iter_mut()
            .map(|step| (step.order(), step.revision(&input)))
            .collect();
        let triggers = Triggers {
            faithful: faithful.id(),
            faithful_hash,
            size,
            animation: state.map(|state| state.animation.frame),
            flash: state.map(|state| {
                (
                    state.presentation_camp_flash_inverted,
                    state.presentation_stats_row_flash,
                )
            }),
            window,
            side_frame: side.and_then(|side| side.frame),
            steps: revisions,
        };
        if host.last.as_ref() == Some(&triggers) {
            // Nothing changed; the sprite may still need its copy (the
            // intro handed its handle to play, or a sprite was respawned).
            if let Some(presented) = &host.presented {
                retarget_sprites(&mut sprites, &faithful, presented);
            }
            return;
        }
        host.source.clear();
        host.source.extend_from_slice(bytes);
        triggers
    };

    // Compose on the copy.
    let input = PresentInput {
        faithful: &host.source,
        size: triggers.size,
        faithful_hash: triggers.faithful_hash,
        state,
        atlas,
        font,
        window,
        scale_factor,
        side,
        sweep_settling,
    };
    host.frame.size = triggers.size;
    host.frame.smooth = false;
    host.frame.map_region = None;
    host.frame.rgba.clear();
    host.frame.rgba.extend_from_slice(&host.source);
    for step in steps.0.iter_mut() {
        step.apply(&input, &mut host.frame);
    }
    host.composes += 1;
    host.last = Some(triggers);

    // One presented image per geometry.
    let geometry = host.frame.geometry();
    let reuse = host
        .presented
        .as_ref()
        .filter(|_| host.geometry == geometry);
    let presented = match reuse.and_then(|handle| images.get_mut(handle).map(|i| (handle, i))) {
        Some((handle, image)) => {
            image.data = Some(host.frame.rgba.clone());
            handle.clone()
        }
        None => {
            let handle = images.add(presented_image(&host.frame));
            if let Some(old) = host.presented.replace(handle.clone()) {
                retarget_sprites(&mut sprites, &old, &handle);
                images.remove(&old);
            }
            host.geometry = geometry;
            handle
        }
    };
    retarget_sprites(&mut sprites, &faithful, &presented);
}

#[cfg(test)]
pub(crate) mod tests;

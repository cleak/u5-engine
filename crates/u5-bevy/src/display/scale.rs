//! The scale policies (Phase B design 2.2.11, 2.3.5; build plan M1-02).
//!
//! `display.scale_mode` picks how the presented image fills the window and
//! `display.pixel_aspect` the height of one of its pixels:
//!
//! - **fit**: the largest rect of the image's display aspect inside the
//!   window. With the DOS aspect this is the 1.0.1 picture, and the
//!   faithful `fit_display_to_window` keeps the frame.
//! - **sharp**: the fit size, cut to whole physical pixels and placed on a
//!   whole physical pixel, so M1-03's sharp-bilinear prescale lands exactly.
//! - **integer-square**: the largest whole factor `k` with square pixels,
//!   `k*W x k*H`. The pixel aspect is ignored.
//! - **integer-hybrid**: the largest whole horizontal factor `k` whose
//!   aspect-correct height still fits, `k*W x floor(k*H*aspect)`.
//! - **stretch**: the whole window, with no aspect kept.
//!
//! Everything is worked out in physical pixels and only then turned into
//! Bevy's logical units, so a display scale of 125% or 150% still gives
//! whole physical factors. A window smaller than the image at scale 1 (below
//! 320x240 for the DOS frame) falls back to Fit for as long as it lasts, as
//! does an integer mode that finds no whole factor, which says so once.
//!
//! Only a policy other than Fit at the DOS aspect adds anything to the app
//! ([`add_scale`]): the foundation's frame fit with a [`FramePolicy`] that
//! follows [`Display`]. Under Fit the app is the 1.0.1 app.
//!
//! **Sharp-bilinear** (2.3.5, D2.13, M1-03). Under `sharp` the presented
//! image also goes through the `[prescale]` step of the foundation's
//! pipeline ([`Prescale`]): each axis gets its own whole factor,
//! `clamp(floor(shown / image), 1, 8)`, the image is expanded by those
//! factors with nearest-neighbour, and the copy is shown with a linear
//! sampler, so pixels stay crisp and only their last fraction is blended.
//! The work is done on the CPU on plain RGBA, and only when the frame is
//! recomposed. The faithful image and its nearest sampler are never
//! touched.

use std::sync::atomic::{AtomicBool, Ordering};

use bevy::prelude::*;
use u5_runtime::enhancements::{DisplaySettings, PixelAspect, ScaleMode};

use super::Display;
use super::grow::Canvas;
use crate::enh::present::{
    PresentInput, PresentStep, PresentSteps, PresentedFrame, StepOrder, register_present_step,
};
use crate::enh::side::{
    FrameFit, FramePolicy, SideSpaceSettings, WindowPixels, install_side_space,
};
use crate::{EnhSet, fitted_display_size};

/// The logged line for an integer mode that found no whole factor.
pub const NO_INTEGER_FACTOR: &str =
    "display.scale_mode: no whole scale factor fits this window; using fit";

/// A pixel's height over its width, as a ratio of whole numbers so the
/// integer modes stay exact.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Aspect {
    pub height: u32,
    pub width: u32,
}

impl Aspect {
    /// 1.2, the shell's `DISPLAY_PIXEL_ASPECT`.
    pub const DOS: Aspect = Aspect {
        height: 6,
        width: 5,
    };
    pub const SQUARE: Aspect = Aspect {
        height: 1,
        width: 1,
    };

    pub fn of(aspect: PixelAspect) -> Aspect {
        match aspect {
            PixelAspect::Dos => Aspect::DOS,
            PixelAspect::Square => Aspect::SQUARE,
        }
    }

    pub fn ratio(self) -> f32 {
        self.height as f32 / self.width as f32
    }
}

/// `display.scale_mode` and `display.pixel_aspect` together.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScalePolicy {
    pub mode: ScaleMode,
    pub aspect: PixelAspect,
}

impl ScalePolicy {
    pub fn new(mode: ScaleMode, aspect: PixelAspect) -> Self {
        Self { mode, aspect }
    }

    pub fn of(settings: &DisplaySettings) -> Self {
        Self::new(settings.scale_mode, settings.pixel_aspect)
    }

    /// Fit at the DOS aspect: the 1.0.1 picture, which the faithful fit
    /// draws.
    pub fn is_faithful(self) -> bool {
        self.mode == ScaleMode::Fit && self.aspect == PixelAspect::Dos
    }

    /// The pixel aspect the mode draws with: `integer-square` ignores the
    /// setting.
    pub fn pixel_aspect(self) -> Aspect {
        self.pixel_aspect_for(Aspect::of(self.aspect))
    }

    /// The pixel aspect the mode draws an image of pixel aspect `image`
    /// with: `image` itself, except under `integer-square`.
    pub fn pixel_aspect_for(self, image: Aspect) -> Aspect {
        match self.mode {
            ScaleMode::IntegerSquare => Aspect::SQUARE,
            _ => image,
        }
    }

    /// Where an `image`-sized frame goes in a `window` of physical pixels.
    /// A mode that falls back is Fit at the chosen pixel aspect, so
    /// `integer-square` at the DOS aspect falls back to the 1.0.1 frame.
    /// The small-window rule measures the image at the chosen aspect too
    /// (320x240 for the DOS frame, whatever the mode).
    pub fn scale(self, window: UVec2, image: UVec2) -> ScaledFrame {
        self.scale_at(window, image, Aspect::of(self.aspect))
    }

    /// [`ScalePolicy::scale`] for an image whose pixels are `chosen` high
    /// over wide, in place of the setting's: the grow-left canvas on a
    /// 1080-line window has 1.25 (2.2.11).
    pub fn scale_at(self, window: UVec2, image: UVec2, chosen: Aspect) -> ScaledFrame {
        if !fits_at_one(window, image, chosen) {
            return fit(window, image, chosen);
        }
        let scaled = scale_frame(self.mode, self.pixel_aspect_for(chosen), window, image);
        if scaled.mode == ScaleMode::Fit && self.mode != ScaleMode::Fit {
            return fit(window, image, chosen);
        }
        scaled
    }
}

/// A frame's place in the window, in physical pixels from the top-left
/// corner, and the mode that placed it (Fit when the chosen one fell back).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScaledFrame {
    pub origin: Vec2,
    pub size: Vec2,
    pub mode: ScaleMode,
    /// The whole factor an integer mode used, per axis where it is whole
    /// (`integer-hybrid` is whole across only).
    pub factor: Option<u32>,
}

impl ScaledFrame {
    /// The same place in logical pixels, for a display scale of
    /// `scale_factor`.
    pub fn logical(&self, scale_factor: f32) -> Rect {
        Rect::from_corners(
            self.origin / scale_factor,
            (self.origin + self.size) / scale_factor,
        )
    }
}

/// The scale step on its own: `mode` at `aspect` for an `image` in a
/// `window`, both in physical pixels.
pub fn scale_frame(mode: ScaleMode, aspect: Aspect, window: UVec2, image: UVec2) -> ScaledFrame {
    let (wx, wy) = (u64::from(window.x), u64::from(window.y));
    let (iw, ih) = (u64::from(image.x), u64::from(image.y));
    let (num, den) = (u64::from(aspect.height), u64::from(aspect.width));
    // The image at scale 1 must fit, or every mode is Fit.
    let mode = if fits_at_one(window, image, aspect) {
        mode
    } else {
        ScaleMode::Fit
    };
    let whole = |w: u64, h: u64, factor: Option<u64>| {
        let (w, h) = (w as f32, h as f32);
        ScaledFrame {
            origin: Vec2::new(
                ((wx as f32 - w) / 2.0).floor(),
                ((wy as f32 - h) / 2.0).floor(),
            ),
            size: Vec2::new(w, h),
            mode,
            factor: factor.map(|k| k as u32),
        }
    };
    match mode {
        ScaleMode::Fit => fit(window, image, aspect),
        ScaleMode::Sharp => {
            if wx * ih * num >= wy * iw * den {
                whole((wy * iw * den) / (ih * num), wy, None)
            } else {
                whole(wx, (wx * ih * num) / (iw * den), None)
            }
        }
        ScaleMode::IntegerSquare => {
            let k = (wx / iw).min(wy / ih);
            if k == 0 {
                fit(window, image, aspect)
            } else {
                whole(k * iw, k * ih, Some(k))
            }
        }
        ScaleMode::IntegerHybrid => {
            let k = (wx / iw).min((wy * den) / (ih * num));
            if k == 0 {
                fit(window, image, aspect)
            } else {
                whole(k * iw, (k * ih * num) / den, Some(k))
            }
        }
        ScaleMode::Stretch => whole(wx, wy, None),
    }
}

/// Whether `image` at scale 1 and `aspect` fits in `window`.
fn fits_at_one(window: UVec2, image: UVec2, aspect: Aspect) -> bool {
    let (wx, wy) = (u64::from(window.x), u64::from(window.y));
    let (iw, ih) = (u64::from(image.x), u64::from(image.y));
    iw > 0 && ih > 0 && wx >= iw && wy * u64::from(aspect.width) >= ih * u64::from(aspect.height)
}

/// The largest rect of `image`'s display aspect inside `window`, centred,
/// in unrounded physical pixels: the 1.0.1 fit at the DOS aspect.
fn fit(window: UVec2, image: UVec2, aspect: Aspect) -> ScaledFrame {
    let window = window.as_vec2();
    let display = Vec2::new(image.x as f32, image.y as f32 * aspect.ratio());
    let size = if image.x == 0 || image.y == 0 || window.x <= 0.0 || window.y <= 0.0 {
        Vec2::ZERO
    } else {
        let width = window.x.min(window.y * display.x / display.y);
        Vec2::new(width, width * display.y / display.x)
    };
    ScaledFrame {
        origin: (window - size) / 2.0,
        size,
        mode: ScaleMode::Fit,
        factor: None,
    }
}

/// The scale policy as the foundation's frame fit uses it. An integer mode
/// that falls back to Fit says so once a run.
#[derive(Debug, Default)]
pub struct ScaleFit {
    policy: ScalePolicy,
    /// The grow-left layout is on, so the image may be its canvas.
    canvas: bool,
    warned: AtomicBool,
}

impl ScaleFit {
    pub fn new(policy: ScalePolicy) -> Self {
        Self {
            policy,
            canvas: false,
            warned: AtomicBool::new(false),
        }
    }

    /// The same policy, also placing the grow-left canvas (2.3.5): an image
    /// the size of [`Canvas::for_window`] is placed at the canvas's own
    /// pixel aspect under every mode, Fit included, since the faithful fit
    /// knows only the 320x200 frame.
    pub fn with_canvas(mut self, canvas: bool) -> Self {
        self.canvas = canvas;
        self
    }

    /// The canvas `image` is, in this window.
    fn canvas_of(&self, window: WindowPixels, image: UVec2) -> Option<Canvas> {
        if !self.canvas {
            return None;
        }
        Canvas::for_window(window.physical.x, window.physical.y, self.policy.aspect)
            .filter(|canvas| UVec2::new(canvas.w, canvas.h) == image)
    }
}

impl FrameFit for ScaleFit {
    fn place(&self, window: WindowPixels, image: UVec2) -> Option<Rect> {
        if let Some(canvas) = self.canvas_of(window, image) {
            let scaled = self.policy.scale_at(window.physical, image, canvas.aspect);
            return Some(scaled.logical(window.scale_factor));
        }
        if self.policy.is_faithful() {
            return None;
        }
        let scaled = self.policy.scale(window.physical, image);
        let integer = matches!(
            self.policy.mode,
            ScaleMode::IntegerSquare | ScaleMode::IntegerHybrid
        );
        let fell_back = scaled.mode == ScaleMode::Fit && self.policy.mode != ScaleMode::Fit;
        if integer && fell_back && !self.warned.swap(true, Ordering::Relaxed) {
            info!("{NO_INTEGER_FACTOR}");
        }
        // Fit at the DOS aspect, chosen or fallen back to, is the 1.0.1
        // frame, which the faithful fit draws.
        if scaled.mode == ScaleMode::Fit && self.policy.aspect == PixelAspect::Dos {
            return None;
        }
        Some(scaled.logical(window.scale_factor))
    }
}

/// Adds the frame fit for a policy other than the 1.0.1 one; adds nothing
/// under Fit at the DOS aspect. Called from [`super::DisplayPlugin`].
pub(super) fn add_scale(app: &mut App, settings: &DisplaySettings) {
    let policy = ScalePolicy::of(settings);
    let canvas = settings.grow_left.on;
    if policy.is_faithful() && !canvas {
        return;
    }
    // A no-op when the launch already installed it with the file's
    // `panel_priority` and `panel_guard` (see `super::add_display`).
    install_side_space(app, SideSpaceSettings::default());
    app.insert_resource(FramePolicy(Box::new(
        ScaleFit::new(policy).with_canvas(canvas),
    )))
    .add_systems(
        Update,
        follow_scale_settings
            .run_if(resource_changed::<Display>)
            .in_set(EnhSet::Apply),
    );
    if policy.prescales() {
        register_present_step(app, Prescale::default());
    }
}

impl ScalePolicy {
    /// Whether the mode is sharp-bilinear, so the `[prescale]` step runs.
    pub fn prescales(self) -> bool {
        self.mode == ScaleMode::Sharp
    }
}

/// A changed [`Display`] (the menu overlay or a hotkey) re-sets the policy,
/// so the frame fit picks it up on this frame's layout, and adds or removes
/// the `[prescale]` step while the pipeline is in the app.
fn follow_scale_settings(
    display: Res<Display>,
    mut current: Local<Option<(ScalePolicy, bool)>>,
    mut policy: ResMut<FramePolicy>,
    steps: Option<ResMut<PresentSteps>>,
) {
    let wanted = (ScalePolicy::of(&display.0), display.0.grow_left.on);
    if current.is_none() {
        // The policy `add_scale` installed came from the launch settings.
        *current = Some(wanted);
        return;
    }
    if *current != Some(wanted) {
        let previous = current.replace(wanted).map(|(policy, _)| policy);
        let (wanted, canvas) = wanted;
        policy.0 = Box::new(ScaleFit::new(wanted).with_canvas(canvas));
        if previous == Some(wanted) {
            return;
        }
        if let Some(mut steps) = steps {
            if wanted.prescales() {
                steps.register(Prescale::default());
            } else {
                steps.remove(StepOrder::PRESCALE);
            }
        }
    }
}

/// The largest whole factor the prescale uses on an axis (D2.13).
pub const PRESCALE_CAP: u32 = 8;

/// The prescale's whole factor on each axis for an `image` shown at
/// `shown` physical pixels: `clamp(floor(shown / image), 1, 8)` (2.3.5).
pub fn prescale_factors(shown: Vec2, image: UVec2) -> UVec2 {
    let axis = |shown: f32, image: u32| {
        if image == 0 || !shown.is_finite() || shown <= 0.0 {
            return 1;
        }
        // A shown size a rounding hair under a whole multiple (1599.9998 for
        // 8 x 200) still counts as that multiple.
        let factor = ((shown + 1.0e-3) / image as f32).floor() as u32;
        factor.clamp(1, PRESCALE_CAP)
    };
    UVec2::new(axis(shown.x, image.x), axis(shown.y, image.y))
}

/// `rgba`, `size` pixels, expanded `factors` times on each axis by
/// nearest-neighbour into `out`: every source pixel becomes a
/// `factors.x x factors.y` block of itself.
pub fn prescale_nearest(rgba: &[u8], size: UVec2, factors: UVec2, out: &mut Vec<u8>) {
    let (width, height) = (size.x as usize, size.y as usize);
    let (kx, ky) = (factors.x.max(1) as usize, factors.y.max(1) as usize);
    let row_bytes = width * kx * 4;
    out.clear();
    out.reserve(row_bytes * height * ky);
    let mut row = Vec::with_capacity(row_bytes);
    for source in rgba.chunks_exact(width * 4).take(height) {
        row.clear();
        for pixel in source.as_chunks::<4>().0 {
            for _ in 0..kx {
                row.extend_from_slice(pixel);
            }
        }
        for _ in 0..ky {
            out.extend_from_slice(&row);
        }
    }
}

/// The physical size the presented image is shown at: the arbiter's frame
/// while it places the frame, otherwise the 1.0.1 fit of the window.
fn shown_size(input: &PresentInput, image: UVec2) -> Vec2 {
    match input.side.and_then(|side| side.frame) {
        Some(frame) => frame.size() * input.scale_factor,
        None => fitted_display_size(image.x, image.y, input.window.as_vec2()),
    }
}

/// The `[prescale]` step (2.3.5): last in the pipeline, it expands whatever
/// the earlier steps made (the 320x200 frame, or the grow-left canvas) by
/// [`prescale_factors`] and asks for the linear sampler.
#[derive(Debug, Default)]
pub struct Prescale {
    scratch: Vec<u8>,
}

impl PresentStep for Prescale {
    fn order(&self) -> StepOrder {
        StepOrder::PRESCALE
    }

    fn revision(&mut self, input: &PresentInput) -> u64 {
        // The window and the layout are triggers of their own; the shown
        // size covers a display-scale change that moves neither.
        let shown = shown_size(input, input.size);
        (u64::from(shown.x.to_bits()) << 32) | u64::from(shown.y.to_bits())
    }

    fn apply(&mut self, input: &PresentInput, frame: &mut PresentedFrame) {
        let factors = prescale_factors(shown_size(input, frame.size), frame.size);
        frame.smooth = true;
        if factors == UVec2::ONE {
            return;
        }
        prescale_nearest(&frame.rgba, frame.size, factors, &mut self.scratch);
        std::mem::swap(&mut frame.rgba, &mut self.scratch);
        frame.size *= factors;
    }
}

#[cfg(test)]
mod tests;

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

use std::sync::atomic::{AtomicBool, Ordering};

use bevy::prelude::*;
use u5_runtime::enhancements::{DisplaySettings, PixelAspect, ScaleMode};

use super::Display;
use crate::EnhSet;
use crate::enh::side::{
    FrameFit, FramePolicy, SideSpaceSettings, WindowPixels, install_side_space,
};

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
        match self.mode {
            ScaleMode::IntegerSquare => Aspect::SQUARE,
            _ => Aspect::of(self.aspect),
        }
    }

    /// Where an `image`-sized frame goes in a `window` of physical pixels.
    /// A mode that falls back is Fit at the chosen pixel aspect, so
    /// `integer-square` at the DOS aspect falls back to the 1.0.1 frame.
    /// The small-window rule measures the image at the chosen aspect too
    /// (320x240 for the DOS frame, whatever the mode).
    pub fn scale(self, window: UVec2, image: UVec2) -> ScaledFrame {
        let chosen = Aspect::of(self.aspect);
        if !fits_at_one(window, image, chosen) {
            return fit(window, image, chosen);
        }
        let scaled = scale_frame(self.mode, self.pixel_aspect(), window, image);
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
    warned: AtomicBool,
}

impl ScaleFit {
    pub fn new(policy: ScalePolicy) -> Self {
        Self {
            policy,
            warned: AtomicBool::new(false),
        }
    }
}

impl FrameFit for ScaleFit {
    fn place(&self, window: WindowPixels, image: UVec2) -> Option<Rect> {
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
    if policy.is_faithful() {
        return;
    }
    // A no-op when the launch already installed it with the file's
    // `panel_priority` and `panel_guard` (see `super::add_display`).
    install_side_space(app, SideSpaceSettings::default());
    app.insert_resource(FramePolicy(Box::new(ScaleFit::new(policy))))
        .add_systems(
            Update,
            follow_scale_settings
                .run_if(resource_changed::<Display>)
                .in_set(EnhSet::Apply),
        );
}

/// A changed [`Display`] (the menu overlay or a hotkey) re-sets the policy,
/// so the frame fit picks it up on this frame's layout.
fn follow_scale_settings(
    display: Res<Display>,
    mut current: Local<Option<ScalePolicy>>,
    mut policy: ResMut<FramePolicy>,
) {
    let wanted = ScalePolicy::of(&display.0);
    if current.is_none() {
        // The policy `add_scale` installed came from the launch settings.
        *current = Some(ScalePolicy::of(&display.0));
        return;
    }
    if *current != Some(wanted) {
        *current = Some(wanted);
        policy.0 = Box::new(ScaleFit::new(wanted));
    }
}

#[cfg(test)]
mod tests;

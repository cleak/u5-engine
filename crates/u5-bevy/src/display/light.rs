//! Brightness on the map view (Phase B design chapter 9, 9.3.3; build plan
//! D-M2.2, D-M2.5, M2-03): the `[brightness]` pipeline step.
//!
//! The step scales the light of the presented copy through one
//! [`BrightnessLut`] for the chosen percent, built once when the step is
//! made: R, G and B of each pixel in the region, alpha kept. It runs after
//! `[grow-left compose]` and before `[prescale]` ([`StepOrder::BRIGHTNESS`]),
//! on the CPU, and never touches the faithful image.
//!
//! - **Scope `map`.** Nothing while [`PlayState::light_view_kind`] is
//!   `None` (the intro, title, character creation, full-screen text pages,
//!   the shrine and the cinematic screens). That is the
//!   [`PlayState::map_view_kind`], except that a visibility sweep counts as
//!   the map it sweeps over (the sweep draws that map). Otherwise the copy's
//!   [`MapRegion`] when a step set one (the grow-left canvas: the map
//!   window less the docked column and the crawl's text), or else the
//!   320x200 frame's viewport, pixels 8..=183 on both axes. The frame
//!   border, stats panel and message window keep their exact colours.
//! - **Scope `frame`.** The whole copy, the intro and title included.
//!   Side panels and overlays are separate images and are never dimmed.
//!
//! The step's revision is the percent, the scope and the map-view kind,
//! so leaving the map or a menu change repaints an unchanged faithful
//! frame. It is registered only while `display.brightness` is below 100
//! (9.4.2): at 100 there is no step, table or system, unless the menu
//! overlay could change the key in play, when `follow_light_settings`
//! waits for that change. The vignette (9.3.3 steps 2 and 3) is P3's.
//!
//! [`PlayState::map_view_kind`]: u5_runtime::PlayState::map_view_kind
//! [`PlayState::light_view_kind`]: u5_runtime::PlayState::light_view_kind

use std::hash::{Hash, Hasher};

use bevy::prelude::*;
use u5_runtime::MapViewKind;
use u5_runtime::enhancements::{BrightnessLut, BrightnessScope, DisplayLight, DisplaySettings};

use super::Display;
use crate::EnhSet;
use crate::enh::present::{
    MapRegion, PresentInput, PresentStep, PresentSteps, PresentedFrame, StepOrder,
    install_present_pipeline, register_present_step,
};
use crate::{VISUAL_PLAY_FRAME_HEIGHT, VISUAL_PLAY_FRAME_WIDTH};

/// The 320x200 frame.
pub const FRAME: UVec2 = UVec2::new(VISUAL_PLAY_FRAME_WIDTH, VISUAL_PLAY_FRAME_HEIGHT);

/// The 11x11 viewport on the 320x200 frame, `(x0, y0, x1, y1)`
/// inclusive: pixels 8..=183 on both axes (9.2.2).
pub const VIEWPORT: (u32, u32, u32, u32) = (8, 8, 183, 183);

/// What the step dims on a copy of `size` (9.3.3 step 1): `None` for
/// nothing. `set` is the region a step before it set
/// ([`PresentedFrame::map_region`]).
pub fn brightness_region(
    scope: BrightnessScope,
    kind: Option<MapViewKind>,
    size: UVec2,
    set: Option<&MapRegion>,
) -> Option<MapRegion> {
    match scope {
        BrightnessScope::Frame => Some(MapRegion::rect(
            size,
            (0, 0, size.x.saturating_sub(1), size.y.saturating_sub(1)),
        )),
        BrightnessScope::Map => {
            kind?;
            match set {
                Some(region) if region.size == size => Some(region.clone()),
                Some(_) => None,
                None if size == FRAME => Some(MapRegion::rect(size, VIEWPORT)),
                // A copy some other step resized without saying where the
                // map is: leave it alone rather than guess.
                None => None,
            }
        }
    }
}

/// `lut` over every pixel of `region` on `frame` (R, G and B; alpha
/// kept).
pub fn dim_region(lut: &BrightnessLut, region: &MapRegion, frame: &mut PresentedFrame) {
    let w = frame.size.x as usize;
    let rgba = &mut frame.rgba;
    region.for_each_run(|y, x0, x1| {
        let from = (y as usize * w + x0 as usize) * 4;
        let to = (y as usize * w + x1 as usize) * 4;
        if let Some(run) = rgba.get_mut(from..to) {
            lut.apply_rgba(run);
        }
    });
}

/// The `[brightness]` step for one percent and scope.
pub struct BrightnessStep {
    percent: u8,
    scope: BrightnessScope,
    lut: BrightnessLut,
}

impl BrightnessStep {
    /// The step `light` asks for, or `None` at 100 (off).
    pub fn of(light: &DisplayLight) -> Option<Self> {
        light.brightness_on().then(|| Self {
            percent: light.brightness,
            scope: light.scope,
            lut: BrightnessLut::for_percent(light.brightness),
        })
    }

    /// Its table.
    pub fn lut(&self) -> &BrightnessLut {
        &self.lut
    }

    /// Dims `frame` as the step does for a state showing `kind`.
    pub fn dim(&self, kind: Option<MapViewKind>, frame: &mut PresentedFrame) {
        if let Some(region) =
            brightness_region(self.scope, kind, frame.size, frame.map_region.as_ref())
        {
            dim_region(&self.lut, &region, frame);
        }
    }
}

/// The map the step dims: [`PlayState::light_view_kind`], which counts a
/// visibility sweep as the map it sweeps over, so the map does not flash to
/// full colour during the White potion's sweep.
///
/// [`PlayState::light_view_kind`]: u5_runtime::PlayState::light_view_kind
fn kind_of(input: &PresentInput) -> Option<MapViewKind> {
    input.state.and_then(|state| state.light_view_kind())
}

impl PresentStep for BrightnessStep {
    fn order(&self) -> StepOrder {
        StepOrder::BRIGHTNESS
    }

    fn revision(&mut self, input: &PresentInput) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        (self.percent, self.scope as u8, kind_of(input)).hash(&mut hasher);
        hasher.finish()
    }

    fn apply(&mut self, input: &PresentInput, frame: &mut PresentedFrame) {
        self.dim(kind_of(input), frame);
    }
}

/// Registers the brightness step while `display.brightness` is below 100,
/// with `follow_light_settings` for a change in play. Adds nothing at 100
/// unless the menu overlay is on (it could lower the key in play).
pub(crate) fn add_light(app: &mut App, settings: &DisplaySettings) {
    let menu = app
        .world()
        .get_resource::<crate::enh::Toggles>()
        .is_some_and(|toggles| crate::enh::input::menu_overlay_on(toggles.0));
    let step = BrightnessStep::of(&settings.light);
    if step.is_none() && !menu {
        return;
    }
    match step {
        Some(step) => register_present_step(app, step),
        // The menu may switch it on: the step list has to be there.
        None => install_present_pipeline(app),
    }
    app.add_systems(
        Update,
        follow_light_settings
            .run_if(resource_changed::<Display>)
            .in_set(EnhSet::Apply),
    );
}

/// A changed [`Display`] (the menu overlay) re-registers the step with the
/// new percent and scope, or removes it at 100.
fn follow_light_settings(
    display: Res<Display>,
    mut current: Local<Option<(u8, BrightnessScope)>>,
    steps: Option<ResMut<PresentSteps>>,
) {
    let light = display.0.light;
    let wanted = (light.brightness, light.scope);
    let Some(previous) = current.replace(wanted) else {
        // The step `add_light` registered came from the launch settings.
        return;
    };
    if previous == wanted {
        return;
    }
    if let Some(mut steps) = steps {
        match BrightnessStep::of(&light) {
            Some(step) => steps.register(step),
            None => steps.remove(StepOrder::BRIGHTNESS),
        }
    }
}

#[cfg(test)]
#[path = "light_tests.rs"]
mod tests;

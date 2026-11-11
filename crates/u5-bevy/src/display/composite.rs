//! Sprite compositing (Phase B design chapter 9, 9.3.5-9.3.10; build plan
//! D-M2.2, M2-06): the `[composite]` pipeline step and its sprite masks.
//!
//! The faithful renderer draws each object's whole 16x16 tile opaquely,
//! black box included. This step paints, on the presented copy only, the
//! terrain the cell shows when it is empty into the sprite's *background*
//! black:
//!
//! 1. On a faithful hash change, or an animation tick while the last peek's
//!    `animated_under` was set, it asks [`PlayState::peek_object_cells`]
//!    which viewport cells hold an object, which atlas tile was drawn there
//!    and the 16x16 pixels of the terrain under it. `None` (no top-down or
//!    combat map) passes the image through.
//! 2. For each cell, at `(8 + 16 * col, 8 + 16 * row)` on the 320x200 copy,
//!    a pixel is replaced only where the tile's mask bit is set (the
//!    background, or the background less its 1-px halo with
//!    `display.composite_halo`) **and** the pixel is still exactly palette
//!    0. Anything the faithful frame drew over the sprite afterwards (the
//!    combat cursor, a camp or potion flash) is not black there and is
//!    kept. The terrain goes through the palette the faithful frame uses
//!    ([`crate::visual_palette`]).
//! 3. The revision is a hash of the peek's cells (`col`, `row`, `drawn`,
//!    `under`), the animation phase while `animated_under` is set, and the
//!    halo flag, so water under a ship ripples even when the faithful frame
//!    does not change (9.3.8).
//!
//! The step is first in the pipeline ([`StepOrder::COMPOSITE`]), so it sees
//! the faithful pixels and the grow-left compositor copies a composited
//! core. The grow-left step composites the ring's objects itself, with the
//! same masks and rule (P5b, M2-07).
//!
//! The masks ([`CompositeMasks`]) are built once from the atlas the shell
//! loaded from the player's TILES.16, on the first play frame the step
//! sees, and are never written anywhere. With `display.sprite_composite`
//! off there is no step, no mask resource and no call into the peek
//! (9.4.2), unless the menu overlay is on, when `follow_composite_settings`
//! waits for the key to change.
//!
//! [`PlayState::peek_object_cells`]: u5_runtime::PlayState::peek_object_cells

use std::hash::{Hash, Hasher};
use std::sync::{Arc, OnceLock};

use bevy::prelude::*;
use u5_runtime::enhancements::{DisplaySettings, SpriteMasks, mask_bit};
use u5_runtime::{ObjectCellPeek, PlayState, TILE_ATLAS_SIDE};

use super::Display;
use crate::EnhSet;
use crate::enh::present::{
    PresentInput, PresentStep, PresentSteps, PresentedFrame, StepOrder, install_present_pipeline,
    register_present_step,
};
use crate::{VISUAL_PLAY_FRAME_HEIGHT, VISUAL_PLAY_FRAME_WIDTH};

/// The 320x200 frame the step works on.
pub const FRAME: UVec2 = UVec2::new(VISUAL_PLAY_FRAME_WIDTH, VISUAL_PLAY_FRAME_HEIGHT);

/// The viewport's top-left pixel on the 320x200 frame, on both axes.
pub const VIEWPORT_ORIGIN: usize = 8;

/// The sprite background masks, built once from the player's atlas on the
/// first play frame (9.3.6). Exists only while compositing is (or has been)
/// on.
#[derive(Resource, Clone, Default)]
pub struct CompositeMasks(pub Arc<OnceLock<SpriteMasks>>);

impl CompositeMasks {
    /// The masks, once built.
    pub fn get(&self) -> Option<&SpriteMasks> {
        self.0.get()
    }
}

/// Paints the terrain of each cell of `peek` into the background pixels of
/// the 320x200 RGBA `rgba` that are still exactly palette 0 (9.3.8 steps 2
/// and 3). Returns how many pixels it changed.
pub fn composite_cells(
    rgba: &mut [u8],
    palette: &[[u8; 3]],
    masks: &SpriteMasks,
    peek: &ObjectCellPeek,
    halo: bool,
) -> usize {
    let width = FRAME.x as usize;
    let black = palette[0];
    let mut changed = 0;
    for cell in &peek.cells {
        let mask = masks.background(usize::from(cell.drawn), halo);
        let x0 = VIEWPORT_ORIGIN + TILE_ATLAS_SIDE * usize::from(cell.col);
        let y0 = VIEWPORT_ORIGIN + TILE_ATLAS_SIDE * usize::from(cell.row);
        for y in 0..TILE_ATLAS_SIDE {
            for x in 0..TILE_ATLAS_SIDE {
                if !mask_bit(mask, x, y) {
                    continue;
                }
                let at = ((y0 + y) * width + x0 + x) * 4;
                let Some(pixel) = rgba.get_mut(at..at + 3) else {
                    continue;
                };
                if *pixel != black {
                    continue;
                }
                let index = usize::from(cell.under_pixels[y * TILE_ATLAS_SIDE + x]);
                pixel.copy_from_slice(&palette[index % palette.len()]);
                changed += 1;
            }
        }
    }
    changed
}

/// The animation clocks the terrain under an object moves with: the water
/// scroll, the fire flicker and the shared phase.
fn animation_phase(state: &PlayState) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    (
        state.water_scroll,
        &state.fire_flicker,
        state.animation.frame,
    )
        .hash(&mut hasher);
    hasher.finish()
}

/// The last peek and what it was taken at.
#[derive(Clone, Debug)]
struct PeekCache {
    faithful_hash: u64,
    /// The animation phase, while the peek's `animated_under` was set.
    phase: Option<u64>,
    peek: Option<ObjectCellPeek>,
    revision: u64,
}

/// The `[composite]` step.
pub struct CompositeStep {
    masks: Arc<OnceLock<SpriteMasks>>,
    halo: bool,
    cache: Option<PeekCache>,
    peeks: u64,
}

impl CompositeStep {
    /// The step drawing with `masks` (filled on the first play frame), with
    /// or without the halo.
    pub fn new(masks: Arc<OnceLock<SpriteMasks>>, halo: bool) -> Self {
        Self {
            masks,
            halo,
            cache: None,
            peeks: 0,
        }
    }

    /// Whether it keeps a 1-px black outline.
    pub fn halo(&self) -> bool {
        self.halo
    }

    /// How many times it has called the peek.
    pub fn peeks(&self) -> u64 {
        self.peeks
    }

    /// The peek for `input`: the cached one unless the faithful hash
    /// changed, or the animation phase did while the terrain under an
    /// object animates. `None` before play (no state or atlas yet).
    fn refresh(&mut self, input: &PresentInput) -> Option<&PeekCache> {
        let state = input.state?;
        let atlas = input.atlas?;
        let phase = animation_phase(state);
        let fresh = self.cache.as_ref().is_some_and(|cache| {
            cache.faithful_hash == input.faithful_hash
                && cache.phase.is_none_or(|last| last == phase)
        });
        if !fresh {
            let masks = self.masks.get_or_init(|| SpriteMasks::from_atlas(atlas));
            let peek = state.peek_object_cells(atlas, masks);
            self.peeks += 1;
            let animated = peek.as_ref().is_some_and(|peek| peek.animated_under);
            let phase = animated.then_some(phase);
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            if let Some(peek) = &peek {
                for cell in &peek.cells {
                    (cell.col, cell.row, cell.drawn, cell.under).hash(&mut hasher);
                }
            }
            peek.is_some().hash(&mut hasher);
            phase.hash(&mut hasher);
            self.halo.hash(&mut hasher);
            self.cache = Some(PeekCache {
                faithful_hash: input.faithful_hash,
                phase,
                peek,
                revision: hasher.finish(),
            });
        }
        self.cache.as_ref()
    }
}

impl PresentStep for CompositeStep {
    fn order(&self) -> StepOrder {
        StepOrder::COMPOSITE
    }

    fn revision(&mut self, input: &PresentInput) -> u64 {
        let halo = self.halo;
        match self.refresh(input) {
            Some(cache) => cache.revision,
            // The intro: nothing to draw, one revision per halo setting.
            None => u64::from(halo),
        }
    }

    fn apply(&mut self, input: &PresentInput, frame: &mut PresentedFrame) {
        if frame.size != FRAME {
            return;
        }
        let Some(atlas) = input.atlas else {
            return;
        };
        let halo = self.halo;
        let masks = Arc::clone(&self.masks);
        let Some(peek) = self.refresh(input).and_then(|cache| cache.peek.as_ref()) else {
            return;
        };
        let Some(masks) = masks.get() else {
            return;
        };
        composite_cells(
            &mut frame.rgba,
            crate::visual_palette(atlas.depth),
            masks,
            peek,
            halo,
        );
    }
}

/// The session's [`CompositeMasks`], inserted on first use: the
/// `[composite]` step and the grow-left ring (P5b) draw with the same
/// masks, built once. Called only while compositing is on.
pub(crate) fn shared_masks(app: &mut App) -> CompositeMasks {
    if let Some(masks) = app.world().get_resource::<CompositeMasks>() {
        return masks.clone();
    }
    let masks = CompositeMasks::default();
    app.insert_resource(masks.clone());
    masks
}

/// Registers the compositing step while `display.sprite_composite` is on,
/// with the [`CompositeMasks`] it fills and `follow_composite_settings` for
/// a change in play. Adds nothing while it is off, unless the menu overlay
/// is on (it could switch compositing on in play).
pub(crate) fn add_composite(app: &mut App, settings: &DisplaySettings) {
    let menu = app
        .world()
        .get_resource::<crate::enh::Toggles>()
        .is_some_and(|toggles| crate::enh::input::menu_overlay_on(toggles.0));
    let light = settings.light;
    if !light.composite && !menu {
        return;
    }
    if light.composite {
        let masks = shared_masks(app);
        register_present_step(app, CompositeStep::new(masks.0, light.halo));
    } else {
        // The menu may switch it on: the step list has to be there.
        install_present_pipeline(app);
    }
    app.add_systems(
        Update,
        follow_composite_settings
            .run_if(resource_changed::<Display>)
            .in_set(EnhSet::Apply),
    );
}

/// A changed [`Display`] (the menu overlay) registers the step with the new
/// halo setting, or removes it when compositing went off. The masks, once
/// built, are kept for the session, so switching it back on does not
/// rebuild them.
fn follow_composite_settings(
    mut commands: Commands,
    display: Res<Display>,
    mut current: Local<Option<(bool, bool)>>,
    masks: Option<Res<CompositeMasks>>,
    steps: Option<ResMut<PresentSteps>>,
) {
    let light = display.0.light;
    let wanted = (light.composite, light.halo);
    let Some(previous) = current.replace(wanted) else {
        // The step `add_composite` registered came from the launch settings.
        return;
    };
    if previous == wanted {
        return;
    }
    let Some(mut steps) = steps else {
        return;
    };
    if !light.composite {
        steps.remove(StepOrder::COMPOSITE);
        return;
    }
    let masks = match masks {
        Some(masks) => masks.clone(),
        None => {
            let masks = CompositeMasks::default();
            commands.insert_resource(masks.clone());
            masks
        }
    };
    steps.register(CompositeStep::new(masks.0, light.halo));
}

#[cfg(test)]
#[path = "composite_tests.rs"]
mod tests;

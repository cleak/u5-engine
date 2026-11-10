//! The grow-left layout's canvas (Phase B design 2.2.2, 2.2.11, 2.3.7;
//! build plan M1-08).
//!
//! The grow-left screen is drawn on a canvas of its own, sized from the
//! window by one rule (2.2.11):
//!
//! - **Rows.** 240 at the canvas pixel aspect (1.2 with
//!   `pixel_aspect = dos`), except on a window whose height is a multiple
//!   of 1080, which gets 216 rows at 1.25 so that its 4x5 (or 8x10) scale
//!   is exact. With `pixel_aspect = square` the aspect is 1.
//! - **Width.** `W = 16 * floor(H * aspect * window_w / window_h / 16)`,
//!   worked in whole numbers. The named layouts come out exact: 512x240 at
//!   2560x1440 (5x6), 688x240 at 3440x1440 (5x6) and 480x216 at 1920x1080
//!   (4x5).
//! - **The party** sits exactly mid-canvas horizontally (16:9: x 248..263)
//!   and mid-map-window vertically (y 116..131 on 240 rows, 104..119 on
//!   216). The map window is the canvas minus a 16-row top band, an 8-row
//!   bottom band and 8 columns of border each side.
//! - **Narrow canvases** (D2.21). The open column must never cover the
//!   faithful core (frame x 8..183, copied 176 pixels square round the
//!   party), and the column's right rule is at x 151. Under 480 wide the
//!   map is panned right by the fewest whole tiles that put the core at
//!   x 152 or beyond, in both states. Under 384 wide (narrower than 4:3)
//!   there is no grow-left canvas and the classic frame is shown, as it is
//!   on a window smaller than the frame at scale 1 (320x240).
//!
//! **The compositor** (M1-09, 2.3.5 and 2.3.7 steps 1-4) is the
//! `[grow-left compose]` step of the foundation's presentation pipeline,
//! [`GrowLeftCompose`]. While the frame shows the plain top-down view
//! (the ring peek's predicate) and the window has a canvas, it replaces
//! the presented 320x200 copy with the canvas: the chrome, the ring
//! painted from the tile atlas, the faithful viewport copied byte for byte
//! to the core ([`compose`]), and the flash on the ring only. On a fixed
//! screen, in the intro or on a window too small for a canvas it leaves
//! the faithful frame as it is. The frame fit sizes the canvas: the
//! [`FrameImageSize`] it places is set in `EnhSet::Layout` by
//! [`size_grow_frame`] from the same test the step applies. Nothing here
//! writes the game; the faithful image is never touched.
//!
//! **The bands** (M1-10, 2.2.3, 2.2.6, 2.3.7 steps 5-6) are drawn by
//! [`bands`]: the ribbon, food, gold and the date on the top band, and the
//! sky and wind tabs cropped from the frame at one shared centre. Their
//! snapshot is retaken only when the faithful bytes change (D2.11).

use std::hash::{Hash, Hasher};

use bevy::prelude::*;
use u5_runtime::enhancements::{DisplaySettings, PixelAspect};
use u5_runtime::{PeekLightPolicy, PlayState, TileAtlas};

use super::Display;
use super::scale::Aspect;
use crate::enh::present::{
    PresentInput, PresentStep, PresentSteps, PresentedFrame, StepOrder, register_present_step,
};
use crate::enh::side::{FrameImageSize, layout_side_space};
use crate::{EnhSet, VISUAL_PLAY_FRAME_HEIGHT, VISUAL_PLAY_FRAME_WIDTH, VisualState};

pub mod bands;
pub mod compose;

/// Canvas rows on an ordinary window.
pub const CANVAS_ROWS: u32 = 240;
/// Canvas rows on a window whose height is a multiple of 1080.
pub const CANVAS_ROWS_1080: u32 = 216;
/// The 1080-line pixel aspect, 1.25 (4x5).
pub const ASPECT_1080: Aspect = Aspect {
    height: 5,
    width: 4,
};
/// Canvas widths are whole tiles.
pub const TILE: u32 = 16;
/// The top band: ribbon and sky tab.
pub const TOP_BAND: u32 = 16;
/// The bottom band: the wind tab's rule row.
pub const BOTTOM_BAND: u32 = 8;
/// Border columns each side (7 blue and the white rule).
pub const SIDE_BORDER: u32 = 8;
/// The faithful core's side (frame x 8..183, the 11x11 viewport).
pub const CORE_SIDE: u32 = 176;
/// The party tile's offset inside the core (the viewport's centre tile).
pub const CORE_PARTY_OFFSET: u32 = 80;
/// The first map column the open column leaves visible (its right rule is
/// at x 151).
pub const COLUMN_CLEAR_X: u32 = 152;
/// Canvases narrower than this pan the map (D2.21).
pub const NARROW_BELOW: u32 = 480;
/// Canvases narrower than this use the classic frame (D2.21).
pub const CLASSIC_BELOW: u32 = 384;
/// The smallest window with a canvas: the frame at scale 1 (2.2.11).
pub const MIN_WINDOW: (u32, u32) = (320, 240);

/// The grow-left canvas for one window size (2.3.7).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Canvas {
    /// 512, 688, 480, ...: always whole tiles.
    pub w: u32,
    /// 240 or 216.
    pub h: u32,
    /// Left edge of the party tile, after `pan_tiles`.
    pub party_x: u32,
    /// Top edge of the party tile.
    pub party_y: u32,
    /// Whole tiles the map is panned right on a narrow canvas, in both
    /// states. The optional `centre_party` pan of the open column (2.2.7)
    /// is not part of the canvas; the column adds it.
    pub pan_tiles: u8,
    /// One canvas pixel's height over its width: 1.2, 1.25 on the
    /// 216-row canvas, or 1 with square pixels.
    pub aspect: Aspect,
}

impl Canvas {
    /// The canvas for a window of `window_w` by `window_h` physical
    /// pixels, or `None` where the classic frame is shown instead: a
    /// window below 320x240, or a canvas under 384 wide.
    pub fn for_window(window_w: u32, window_h: u32, pixel_aspect: PixelAspect) -> Option<Self> {
        if window_w < MIN_WINDOW.0 || window_h < MIN_WINDOW.1 {
            return None;
        }
        let line_1080 = window_h.is_multiple_of(1080);
        let h = if line_1080 {
            CANVAS_ROWS_1080
        } else {
            CANVAS_ROWS
        };
        let aspect = match (pixel_aspect, line_1080) {
            (PixelAspect::Square, _) => Aspect::SQUARE,
            (PixelAspect::Dos, true) => ASPECT_1080,
            (PixelAspect::Dos, false) => Aspect::DOS,
        };
        // W = 16 * floor(H * (aspect.height / aspect.width) * ww / wh / 16),
        // in whole numbers so the named layouts are exact.
        let numerator = u64::from(h) * u64::from(aspect.height) * u64::from(window_w);
        let denominator = u64::from(aspect.width) * u64::from(window_h) * u64::from(TILE);
        let tiles = numerator / denominator;
        let w = u32::try_from(tiles * u64::from(TILE)).ok()?;
        if w < CLASSIC_BELOW {
            return None;
        }
        let centred_x = (w - TILE) / 2;
        // The core starts CORE_PARTY_OFFSET left of the party tile; pan by
        // the fewest whole tiles that keep it clear of the open column.
        let clear_party_x = COLUMN_CLEAR_X + CORE_PARTY_OFFSET;
        let pan_tiles = clear_party_x.saturating_sub(centred_x).div_ceil(TILE);
        let map_h = h - TOP_BAND - BOTTOM_BAND;
        Some(Self {
            w,
            h,
            party_x: centred_x + pan_tiles * TILE,
            party_y: TOP_BAND + (map_h - TILE) / 2,
            pan_tiles: pan_tiles as u8,
            aspect,
        })
    }

    /// The black map window inside the white rules, as
    /// `(x0, y0, x1, y1)` inclusive.
    pub fn map_window(&self) -> (u32, u32, u32, u32) {
        (
            SIDE_BORDER,
            TOP_BAND,
            self.w - SIDE_BORDER - 1,
            self.h - BOTTOM_BAND - 1,
        )
    }

    /// Top-left of the faithful core (the 176x176 copy of frame x and y
    /// 8..183), centred on the party tile.
    pub fn core_origin(&self) -> (u32, u32) {
        (
            self.party_x - CORE_PARTY_OFFSET,
            self.party_y - CORE_PARTY_OFFSET,
        )
    }

    /// Whether the party tile is exactly mid-canvas horizontally and
    /// mid-map-window vertically (every canvas without a pan).
    pub fn party_centred(&self) -> bool {
        let (x0, y0, x1, y1) = self.map_window();
        2 * self.party_x + TILE == self.w
            && 2 * self.party_y + TILE == y0 + y1 + 1
            && 2 * self.party_x + TILE == x0 + x1 + 1
    }
}

/// The 320x200 faithful frame.
const FRAME: UVec2 = UVec2::new(VISUAL_PLAY_FRAME_WIDTH, VISUAL_PLAY_FRAME_HEIGHT);

/// The canvas the grow-left layout shows for `state` in a window of
/// `window` physical pixels: `None` in the intro (no state), on a fixed
/// screen (2.2.8: exactly when the ring peek would return `None`), and on a
/// window with no canvas (2.2.11).
pub fn grow_canvas(
    state: Option<&PlayState>,
    window: UVec2,
    pixel_aspect: PixelAspect,
) -> Option<Canvas> {
    if !state?.top_down_world_view_active() {
        return None;
    }
    Canvas::for_window(window.x, window.y, pixel_aspect)
}

/// Whether the flash inverts the faithful viewport on this frame: the
/// shared spell or potion flash hold, or the camp apparition (the test the
/// faithful render applies before its XOR).
pub fn flash_active(state: &PlayState) -> bool {
    state.message_window_flash_hold_active() || state.presentation_camp_flash_inverted
}

/// What the compositor is told besides the frame: the light policy, and
/// for steps 5 and 6 the band snapshot, the font and the colour toggle.
#[derive(Clone, Copy)]
pub struct ComposeOptions<'a> {
    pub light: PeekLightPolicy,
    /// `None` draws steps 1-4 only.
    pub bands: Option<compose::Bands<'a>>,
}

/// Steps 1-6 for one state (2.3.7): the ring peek for the canvas's map
/// window under `options.light`, its raster from `atlas`, and
/// [`compose::compose_grow_left`] over `frame`, the faithful 320x200 RGBA
/// the shell drew from `state`, with the bands of `options`. `None` when
/// the peek returns no ring (a fixed screen). Read-only: `state` is shared
/// and the peek draws nothing from the game's random stream.
pub fn compose_for_state(
    frame: &[u8],
    state: &PlayState,
    atlas: &TileAtlas,
    canvas: &Canvas,
    options: ComposeOptions<'_>,
) -> Option<Vec<u8>> {
    let window = compose::peek_window(canvas);
    let ring = state.peek_top_down_ring(window, options.light)?;
    let raster = state.render_peek_ring(&ring, atlas).ok()?;
    let ring_rgba = crate::tile_viewport_to_visual_rgba(&raster);
    let flash = if flash_active(state) {
        u5_runtime::potion_flash_playback(0)
    } else {
        None
    };
    Some(compose::compose_grow_left(
        frame,
        &ring_rgba,
        window,
        canvas,
        flash,
        options.bands,
    ))
}

/// The shell's own faithful play frame for `state` (the bytes the play
/// upload sites write, with the shell's palette) and its grow-left canvas
/// (the column collapsed, the HP colours on), for review shots and tests
/// outside the crate. Renders the frame, which advances `state` exactly as
/// one shell repaint does; the canvas itself is read-only. `None` for the
/// canvas on a fixed screen.
pub fn render_shell_frame_and_canvas(
    state: &mut PlayState,
    atlas: &TileAtlas,
    fonts: (&u5_runtime::FixedCellFont, &u5_runtime::FixedCellFont),
    game_dir: &std::path::Path,
    canvas: &Canvas,
    light: PeekLightPolicy,
) -> (Vec<u8>, Option<Vec<u8>>) {
    let ctx = crate::PlayFrameContext {
        ibm: fonts.0,
        runes: fonts.1,
        game_dir,
        cursor_frame: 0,
    };
    let frame = crate::render_visual_play_frame(state, atlas, ctx);
    let snapshot = bands::band_snapshot(state);
    let options = ComposeOptions {
        light,
        bands: Some(compose::Bands {
            snapshot: &snapshot,
            font: fonts.0,
            hp_colours: true,
            open: false,
        }),
    };
    let canvas = compose_for_state(&frame, state, atlas, canvas, options);
    (frame, canvas)
}

/// The settings the `[grow-left compose]` step follows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GrowComposeSettings {
    /// `display.grow_left_light`.
    pub light: PeekLightPolicy,
    /// `display.pixel_aspect`, which picks the canvas.
    pub pixel_aspect: PixelAspect,
    /// `display.ribbon_hp_colours`.
    pub hp_colours: bool,
}

impl GrowComposeSettings {
    pub fn of(settings: &DisplaySettings) -> Self {
        Self {
            light: settings.grow_left.light,
            pixel_aspect: settings.pixel_aspect,
            hp_colours: settings.grow_left.ribbon_hp_colours,
        }
    }
}

/// The `[grow-left compose]` step (2.3.5): the canvas in place of the
/// faithful frame while the top-down view shows, the faithful frame
/// otherwise.
#[derive(Clone, Debug)]
pub struct GrowLeftCompose {
    pub settings: GrowComposeSettings,
    /// The band snapshot and the faithful hash it was taken at (D2.11): it
    /// is retaken only when the faithful bytes change, so a recompose for
    /// the ring alone never shows a newer value than the original screen.
    bands: Option<(u64, bands::BandSnapshot)>,
}

impl GrowLeftCompose {
    pub fn of(settings: &DisplaySettings) -> Self {
        Self {
            settings: GrowComposeSettings::of(settings),
            bands: None,
        }
    }

    /// The band snapshot for this compose: the cached one while the
    /// faithful bytes are unchanged, a new one otherwise.
    fn bands_for(&mut self, state: &PlayState, faithful_hash: u64) -> &bands::BandSnapshot {
        if self.bands.as_ref().map(|(hash, _)| *hash) != Some(faithful_hash) {
            self.bands = Some((faithful_hash, bands::band_snapshot(state)));
        }
        &self.bands.as_ref().expect("just taken").1
    }
}

/// What the ring shows beyond the faithful frame's own bytes: the party's
/// place, the objects, the light and the clocks the ring's water and fire
/// animate by. The host recomposes on the faithful hash and the shared
/// animation phase already; this catches a change only the ring shows (a
/// townsman moving outside the core while the core stands still).
fn ring_digest(state: &PlayState) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    state.top_down_world_view_active().hash(&mut hasher);
    (state.player.x, state.player.y, state.current_floor()).hash(&mut hasher);
    state.active_objects.hash(&mut hasher);
    (state.ambient_light, state.turn).hash(&mut hasher);
    state.local_light_mask.hash(&mut hasher);
    state.water_scroll.hash(&mut hasher);
    state.fire_flicker.hash(&mut hasher);
    flash_active(state).hash(&mut hasher);
    hasher.finish()
}

impl PresentStep for GrowLeftCompose {
    fn order(&self) -> StepOrder {
        StepOrder::GROW_LEFT_COMPOSE
    }

    fn revision(&mut self, input: &PresentInput) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        let settings = self.settings;
        (
            settings.light as u8,
            settings.pixel_aspect as u8,
            settings.hp_colours,
        )
            .hash(&mut hasher);
        input.state.map(ring_digest).hash(&mut hasher);
        hasher.finish()
    }

    fn apply(&mut self, input: &PresentInput, frame: &mut PresentedFrame) {
        let (Some(state), Some(atlas), Some(font)) = (input.state, input.atlas, input.font) else {
            return;
        };
        if frame.size != FRAME {
            return;
        }
        let settings = self.settings;
        let Some(canvas) = grow_canvas(Some(state), input.window, settings.pixel_aspect) else {
            return;
        };
        let snapshot = self.bands_for(state, input.faithful_hash);
        let options = ComposeOptions {
            light: settings.light,
            bands: Some(compose::Bands {
                snapshot,
                font,
                hp_colours: settings.hp_colours,
                // The docked column is M1-13's.
                open: false,
            }),
        };
        let Some(rgba) = compose_for_state(&frame.rgba, state, atlas, &canvas, options) else {
            return;
        };
        frame.size = UVec2::new(canvas.w, canvas.h);
        frame.rgba = rgba;
    }
}

/// Adds the compositor while `display.grow_left` is on at launch: the
/// step, the [`FrameImageSize`] the frame fit places and the two systems
/// that keep them. Adds nothing while it is off. The frame fit itself (the
/// arbiter and a canvas-aware [`super::scale::ScaleFit`]) comes from
/// [`super::scale::add_scale`].
pub(super) fn add_grow(app: &mut App, settings: &DisplaySettings) {
    if !settings.grow_left.on {
        return;
    }
    app.init_resource::<FrameImageSize>();
    register_present_step(app, GrowLeftCompose::of(settings));
    app.add_systems(
        Update,
        (
            follow_grow_settings
                .run_if(resource_changed::<Display>)
                .in_set(EnhSet::Apply),
            size_grow_frame
                .before(layout_side_space)
                .in_set(EnhSet::Layout),
        ),
    );
}

/// The image the frame fit places this frame: the canvas when the step
/// will show it, otherwise the 320x200 frame. Runs in `EnhSet::Layout`
/// before the layout is solved; nothing between it and the pipeline in
/// `EnhSet::Present` changes what [`grow_canvas`] tests.
pub(crate) fn size_grow_frame(
    display: Res<Display>,
    visual: Option<Res<VisualState>>,
    windows: Query<&Window>,
    mut size: ResMut<FrameImageSize>,
) {
    let window = windows
        .single()
        .map(|window| window.physical_size())
        .unwrap_or(UVec2::ZERO);
    let wanted = display
        .0
        .grow_left
        .on
        .then(|| {
            grow_canvas(
                visual.as_deref().map(|visual| &visual.state),
                window,
                display.0.pixel_aspect,
            )
        })
        .flatten()
        .map(|canvas| UVec2::new(canvas.w, canvas.h));
    if size.0 != wanted {
        size.0 = wanted;
    }
}

/// A changed [`Display`] (the menu overlay) re-registers the step with the
/// new light policy and pixel aspect, or removes it when `grow_left` went
/// off.
fn follow_grow_settings(
    display: Res<Display>,
    mut current: Local<Option<(bool, GrowComposeSettings)>>,
    steps: Option<ResMut<PresentSteps>>,
) {
    let wanted = (display.0.grow_left.on, GrowComposeSettings::of(&display.0));
    let Some(previous) = current.replace(wanted) else {
        // The step `add_grow` registered came from the launch settings.
        return;
    };
    if previous == wanted {
        return;
    }
    if let Some(mut steps) = steps {
        if wanted.0 {
            steps.register(GrowLeftCompose::of(&display.0));
        } else {
            steps.remove(StepOrder::GROW_LEFT_COMPOSE);
        }
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod compose_tests;

#[cfg(test)]
mod bands_tests;

//! The side fill (Phase B design 2.2.1, 2.2.11, 2.3.3, 2.3.4, D2.7; build
//! plan M1-04).
//!
//! A window wider than the 4:3 frame leaves bars beside it, which 1.0.1
//! leaves black (`ClearColor`). `display.pillar_fill` can fill them:
//!
//! - **solid**: `display.pillar_color`;
//! - **ribbon**: the screen's own blue ribbon, traced round each bar,
//!   painted at run time from the player's `IBM.CH` (the three corner
//!   glyphs the gameplay border carves its bevel with, the fourth mirrored)
//!   in the `ChromePalette::EGA` pens. It is drawn in frame pixels and shown
//!   at the frame's scale, so its band and rule are the border's own width,
//!   and its top and bottom line up with the gameplay border's (frame rows
//!   0 and 191). An 8-pixel black gutter keeps it off the frame;
//! - **bezel**: a PNG the player supplies, decoded by the host (the engine
//!   never sees the path), scaled to cover the window and shown through
//!   the bars.
//!
//! The fill is `bevy_ui` nodes (`ImageNode`), one per bar, under every other
//! node. They never touch the frame image or the frame sprite, so the
//! faithful fit keeps sizing the frame and the game never sees them. The
//! bars come from the solved layout: the arbiter's [`SideLayout`] while it
//! owns the frame (a scale policy or a side panel), otherwise the 1.0.1
//! fit's. That covers every screen that shows the 4:3 frame: the intro,
//! character creation, gameplay, the endgame and the U4 transfer, and later
//! the fixed screens under the grow-left layout.
//!
//! Nothing is in the app unless the fill is visible at launch, or the menu
//! overlay could switch it on ([`add_fill`]); then the one system runs only
//! while a fill is chosen or a fill node is still up. A black fill (`black`,
//! or `solid` with `#000000`) and a bezel the host could not decode have no
//! node at all.

use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use u5_runtime::enhancements::{
    BezelDecode, BezelImage, BezelMailbox, DisplaySettings, PillarFill, ScaleMode,
};
use u5_runtime::{CH_CELL_SIDE, ChromePalette, EGA_PALETTE_RGB, FixedCellFont};

use super::Display;
use super::scale::{prescale_factors, prescale_nearest};
use crate::enh::side::SideLayout;
use crate::enh::toast::Toasts;
use crate::{
    EnhSet, VISUAL_PLAY_FRAME_HEIGHT, VISUAL_PLAY_FRAME_WIDTH, VisualIntroState, VisualState,
    fitted_display_size,
};

/// The note for a bezel chosen in play that the host could not use.
pub const BEZEL_REFUSED: &str = "That bezel picture could not be used; the bars stay black.";

/// The black strip, in frame pixels, between the ribbon and the frame.
pub const RIBBON_GUTTER: u32 = 8;
/// The ribbon's band width in frame pixels: the gameplay border's (its
/// bands are seven pixels, with the rule on the eighth).
pub const RIBBON_BAND: u32 = 7;
/// The last frame row of the ribbon: the gameplay border's bottom row
/// (`CHROME_BOTTOM_Y`).
pub const RIBBON_BOTTOM_ROW: u32 = u5_runtime::CHROME_BOTTOM_Y as u32;
/// The narrowest ribbon box, in frame pixels: two corner cells and a
/// cell of rule between them.
pub const RIBBON_MIN_BOX: u32 = 3 * CH_CELL_SIDE as u32;

/// The `IBM.CH` corner glyphs the gameplay border carves its bevel with:
/// top left, top right and bottom left. The bottom right is the bottom
/// left mirrored.
const CORNER_TOP_LEFT: u8 = 0x7b;
const CORNER_TOP_RIGHT: u8 = 0x7c;
const CORNER_BOTTOM_LEFT: u8 = 0x7d;

/// Which bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BarSide {
    Left,
    Right,
}

/// The frame and the bars beside it, in logical window pixels from the
/// top-left corner.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FillGeometry {
    pub frame: Rect,
    pub left: Option<Rect>,
    pub right: Option<Rect>,
}

impl FillGeometry {
    pub fn bar(&self, side: BarSide) -> Option<Rect> {
        match side {
            BarSide::Left => self.left,
            BarSide::Right => self.right,
        }
    }

    /// Logical pixels per frame pixel, across and down.
    pub fn frame_pixel(&self) -> Vec2 {
        self.frame.size()
            / Vec2::new(
                VISUAL_PLAY_FRAME_WIDTH as f32,
                VISUAL_PLAY_FRAME_HEIGHT as f32,
            )
    }
}

/// Bars narrower than this, in logical pixels, are left alone: a rounding
/// sliver is not a bar.
const MIN_BAR: f32 = 1.0;

/// Where the bars are in a `window` of logical pixels: the arbiter's while
/// it owns the frame, otherwise beside the 1.0.1 fit, centred. `None` for a
/// minimised window.
pub fn fill_geometry(window: Vec2, layout: Option<&SideLayout>) -> Option<FillGeometry> {
    if window.x <= 0.0 || window.y <= 0.0 {
        return None;
    }
    let frame = match layout.and_then(|layout| layout.frame) {
        Some(frame) => frame,
        None => {
            let size =
                fitted_display_size(VISUAL_PLAY_FRAME_WIDTH, VISUAL_PLAY_FRAME_HEIGHT, window);
            let min = (window - size) / 2.0;
            Rect::from_corners(min, min + size)
        }
    };
    if frame.width() <= 0.0 || frame.height() <= 0.0 {
        return None;
    }
    let left = (frame.min.x >= MIN_BAR)
        .then(|| Rect::from_corners(Vec2::ZERO, Vec2::new(frame.min.x, window.y)));
    let right = (window.x - frame.max.x >= MIN_BAR)
        .then(|| Rect::from_corners(Vec2::new(frame.max.x, 0.0), window));
    Some(FillGeometry { frame, left, right })
}

/// A ribbon bar's image: its size in frame pixels and the frame row its
/// box starts on (the frame's top, which is below the bar's top when the
/// window is taller than the frame).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RibbonBar {
    pub side: BarSide,
    pub size: UVec2,
    pub top: u32,
}

impl RibbonBar {
    /// The ribbon image for `side` of `geometry`, or `None` when the bar is
    /// too small for a ribbon box (it then stays black).
    pub fn of(geometry: &FillGeometry, side: BarSide) -> Option<(RibbonBar, Rect)> {
        let bar = geometry.bar(side)?;
        let pixel = geometry.frame_pixel();
        let width = (bar.width() / pixel.x).floor() as u32;
        let height = (bar.height() / pixel.y).round() as u32;
        let top = ((geometry.frame.min.y - bar.min.y) / pixel.y)
            .round()
            .max(0.0) as u32;
        if width < RIBBON_MIN_BOX + 2 * RIBBON_GUTTER || top + RIBBON_BOTTOM_ROW >= height {
            return None;
        }
        // The node is the image at the frame's scale, against the frame,
        // so the gutter is exact; the outer margin takes the fraction.
        let node_width = width as f32 * pixel.x;
        let (min_x, max_x) = match side {
            BarSide::Left => (geometry.frame.min.x - node_width, geometry.frame.min.x),
            BarSide::Right => (geometry.frame.max.x, geometry.frame.max.x + node_width),
        };
        let node = Rect::new(min_x, bar.min.y, max_x, bar.max.y);
        let ribbon = RibbonBar {
            side,
            size: UVec2::new(width, height),
            top,
        };
        Some((ribbon, node))
    }

    /// The box's columns, inclusive: the gutter on the frame side and the
    /// same margin on the outer side (the image is the bar's whole frame
    /// pixels, against the frame; what is left of the bar stays black).
    fn box_columns(&self) -> (u32, u32) {
        (RIBBON_GUTTER, self.size.x - RIBBON_GUTTER - 1)
    }
}

/// Paints a ribbon bar in RGBA: black, with a closed ribbon box (the
/// border's 7-pixel blue band, its four corners carved with `font`'s corner
/// glyphs, and the 1-pixel white rule inside the band) from frame row 0 to
/// row 191. A glyph the font lacks paints solid band.
pub fn paint_ribbon_bar(font: &FixedCellFont, bar: RibbonBar, palette: ChromePalette) -> Vec<u8> {
    let (width, height) = (bar.size.x as usize, bar.size.y as usize);
    let mut rgba = pen(palette.background).repeat(width * height);
    let (left, right) = bar.box_columns();
    let (left, right) = (left as usize, right as usize);
    let (top, bottom) = (bar.top as usize, (bar.top + RIBBON_BOTTOM_ROW) as usize);
    let band = RIBBON_BAND as usize;
    let mut fill = |x0: usize, y0: usize, x1: usize, y1: usize, index: u8| {
        for y in y0..=y1.min(height - 1) {
            for x in x0..=x1.min(width - 1) {
                let offset = (y * width + x) * 4;
                rgba[offset..offset + 4].copy_from_slice(&pen(index));
            }
        }
    };
    // The bands, as the border fills its own.
    fill(left, top, right, top + band - 1, palette.chrome);
    fill(left, bottom + 1 - band, right, bottom, palette.chrome);
    fill(left, top, left + band - 1, bottom, palette.chrome);
    fill(right + 1 - band, top, right, bottom, palette.chrome);
    // The rule, one pixel inside the band.
    let (rule_left, rule_right) = (left + band, right - band);
    let (rule_top, rule_bottom) = (top + band, bottom - band);
    let cell = CH_CELL_SIDE;
    let corners = [
        (CORNER_TOP_LEFT, false, left, top),
        (CORNER_TOP_RIGHT, false, right + 1 - cell, top),
        (CORNER_BOTTOM_LEFT, false, left, bottom + 1 - cell),
        (
            CORNER_BOTTOM_LEFT,
            true,
            right + 1 - cell,
            bottom + 1 - cell,
        ),
    ];
    // The corners, stamped opaquely so their clear bits carve the bevel.
    for (code, mirrored, x0, y0) in corners {
        for row in 0..cell {
            let Some(mut bits) = font.glyph_row(code, row) else {
                continue;
            };
            if mirrored {
                bits = bits.reverse_bits();
            }
            for column in 0..cell {
                let index = if bits & (0x80 >> column) != 0 {
                    palette.chrome
                } else {
                    palette.background
                };
                let offset = ((y0 + row) * width + x0 + column) * 4;
                rgba[offset..offset + 4].copy_from_slice(&pen(index));
            }
        }
    }
    let mut fill = |x0: usize, y0: usize, x1: usize, y1: usize| {
        for y in y0..=y1 {
            for x in x0..=x1 {
                let offset = (y * width + x) * 4;
                rgba[offset..offset + 4].copy_from_slice(&pen(palette.accent));
            }
        }
    };
    fill(rule_left, rule_top, rule_right, rule_top);
    fill(rule_left, rule_bottom, rule_right, rule_bottom);
    fill(rule_left, rule_top, rule_left, rule_bottom);
    fill(rule_right, rule_top, rule_right, rule_bottom);
    rgba
}

/// An EGA pen as an opaque RGBA pixel.
fn pen(index: u8) -> [u8; 4] {
    let [r, g, b] = EGA_PALETTE_RGB[usize::from(index & 0x0f)];
    [r, g, b, 0xff]
}

/// The part of a `bezel` image, in its own pixels, that shows through `bar`
/// when the image covers a `window` of logical pixels: scaled until it
/// fills the window both ways, kept to its aspect, centred and cropped.
pub fn bezel_crop(bezel: UVec2, window: Vec2, bar: Rect) -> Rect {
    let size = bezel.as_vec2();
    let scale = (window.x / size.x).max(window.y / size.y);
    let origin = (window - size * scale) / 2.0;
    let min = ((bar.min - origin) / scale).clamp(Vec2::ZERO, size);
    let max = ((bar.max - origin) / scale).clamp(Vec2::ZERO, size);
    Rect::from_corners(min, max)
}

/// Whether `fill` draws anything: black, `solid` black and a bezel the
/// host could not decode draw nothing.
pub fn fill_visible(fill: &PillarFill) -> bool {
    match fill {
        PillarFill::Black => false,
        PillarFill::Solid(colour) => *colour != [0, 0, 0],
        PillarFill::Ribbon => true,
        PillarFill::Bezel(image) => image.is_some(),
    }
}

/// The host's handoff for a bezel chosen in play. Exists only while the
/// host gave one.
#[derive(Resource, Clone, Debug)]
pub struct BezelUpdates(pub BezelMailbox);

/// What the fill system keeps between frames: the bezel in use (from the
/// launch or the host's latest post) and the images it built.
#[derive(Resource, Debug, Default)]
pub struct FillState {
    bezel: Option<BezelImage>,
    bezel_image: Option<Handle<Image>>,
    ribbons: Vec<(RibbonBar, UVec2, Handle<Image>)>,
}

impl FillState {
    pub fn bezel(&self) -> Option<&BezelImage> {
        self.bezel.as_ref()
    }
}

/// A fill node: one per bar.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PillarFillNode {
    pub side: BarSide,
}

/// Adds the fill when it is visible at launch, or when the menu overlay
/// could switch it on later; otherwise adds nothing. Called from
/// [`super::DisplayPlugin`].
pub(super) fn add_fill(app: &mut App, settings: &DisplaySettings, updates: Option<BezelMailbox>) {
    let menu = app
        .world()
        .get_resource::<crate::enh::Toggles>()
        .is_some_and(|toggles| crate::enh::input::menu_overlay_on(toggles.0));
    let chosen = !matches!(settings.pillar_fill, PillarFill::Black);
    if !chosen && !menu {
        return;
    }
    let bezel = match &settings.pillar_fill {
        PillarFill::Bezel(image) => image.clone(),
        _ => None,
    };
    app.insert_resource(FillState { bezel, ..default() });
    if let Some(updates) = updates {
        // A bezel the host refuses in play says so with a note.
        crate::enh::toast::install_shell_services(app);
        app.insert_resource(BezelUpdates(updates));
    }
    app.add_systems(
        Update,
        draw_pillar_fill.run_if(fill_active).in_set(EnhSet::Present),
    );
}

/// A fill is chosen, or a fill node is still up and has to go.
fn fill_active(display: Res<Display>, nodes: Query<(), With<PillarFillNode>>) -> bool {
    !matches!(display.0.pillar_fill, PillarFill::Black) || !nodes.is_empty()
}

/// What one bar's node shows.
#[derive(Clone, Debug, PartialEq)]
enum Look {
    Solid(Color),
    Image {
        image: Handle<Image>,
        rect: Option<Rect>,
    },
}

/// The fill, in `EnhSet::Present`: one node per bar showing the chosen
/// fill, placed from the solved layout, under every other node. A bar with
/// nothing to show has no node.
#[allow(clippy::too_many_arguments)]
fn draw_pillar_fill(
    mut commands: Commands,
    display: Res<Display>,
    windows: Query<&Window>,
    layout: Option<Res<SideLayout>>,
    visual: Option<Res<VisualState>>,
    intro: Option<Res<VisualIntroState>>,
    updates: Option<Res<BezelUpdates>>,
    toasts: Option<ResMut<Toasts>>,
    mut state: ResMut<FillState>,
    mut images: ResMut<Assets<Image>>,
    mut nodes: Query<(Entity, &PillarFillNode, &mut Node, &mut ImageNode)>,
) {
    if let Some(decode) = updates.as_deref().and_then(|updates| updates.0.take()) {
        state.bezel_image = None;
        state.bezel = match decode {
            BezelDecode::Image(image) => Some(image),
            BezelDecode::Failed(reason) => {
                warn!("display.bezel_image: {reason}; the bars stay black");
                if let Some(mut toasts) = toasts {
                    toasts.note(BEZEL_REFUSED);
                }
                None
            }
        };
    }
    // A decoded bezel put in the settings (the launch's) is taken when the
    // settings change; otherwise the latest the host posted stays in use.
    if display.is_changed()
        && let PillarFill::Bezel(Some(image)) = &display.0.pillar_fill
        && state.bezel.as_ref() != Some(image)
    {
        state.bezel = Some(image.clone());
        state.bezel_image = None;
    }
    let fill = match &display.0.pillar_fill {
        PillarFill::Bezel(_) => PillarFill::Bezel(state.bezel.clone()),
        other => other.clone(),
    };
    let window = windows.single().ok();
    let scale_factor = window.map_or(1.0, |window| window.resolution.scale_factor());
    let window =
        window.map(|window| Vec2::new(window.resolution.width(), window.resolution.height()));
    // Under sharp-bilinear the ribbon is drawn the way the frame is: whole
    // steps first, smoothed only in the last fraction (2.3.5).
    let sharp = display.0.scale_mode == ScaleMode::Sharp;
    let geometry = window
        .filter(|_| fill_visible(&fill))
        .and_then(|window| fill_geometry(window, layout.as_deref()));
    let font = visual
        .as_deref()
        .map(|visual| &visual.text_font)
        .or_else(|| {
            intro
                .as_deref()
                .and_then(|intro| intro.font_slots.as_ref())
                .map(|slots| slots.ibm_font())
        });

    let mut wanted: Vec<(BarSide, Rect, Look)> = Vec::new();
    if let (Some(geometry), Some(window)) = (geometry, window) {
        for side in [BarSide::Left, BarSide::Right] {
            let Some(bar) = geometry.bar(side) else {
                continue;
            };
            let look = match &fill {
                PillarFill::Solid([r, g, b]) => (bar, Look::Solid(Color::srgb_u8(*r, *g, *b))),
                PillarFill::Ribbon => {
                    let (Some(font), Some((ribbon, node))) = (font, RibbonBar::of(&geometry, side))
                    else {
                        continue;
                    };
                    let factors = if sharp {
                        prescale_factors(node.size() * scale_factor, ribbon.size)
                    } else {
                        UVec2::ONE
                    };
                    let image = ribbon_image(&mut state, &mut images, font, ribbon, factors);
                    (node, Look::Image { image, rect: None })
                }
                PillarFill::Bezel(Some(bezel)) => {
                    let image = bezel_handle(&mut state, &mut images, bezel);
                    let size = UVec2::new(bezel.width, bezel.height);
                    let rect = Some(bezel_crop(size, window, bar));
                    (bar, Look::Image { image, rect })
                }
                PillarFill::Black | PillarFill::Bezel(None) => continue,
            };
            wanted.push((side, look.0, look.1));
        }
    }
    // Ribbons for bars no longer shown are dropped with their images.
    state
        .ribbons
        .retain(|(ribbon, ..)| wanted.iter().any(|(side, ..)| *side == ribbon.side));
    if !matches!(fill, PillarFill::Bezel(Some(_))) {
        state.bezel_image = None;
    }

    for (entity, fill_node, mut node, mut image_node) in &mut nodes {
        let Some(index) = wanted.iter().position(|(side, ..)| *side == fill_node.side) else {
            commands.entity(entity).despawn();
            continue;
        };
        let (_, rect, look) = wanted.swap_remove(index);
        place(&mut node, rect);
        let shown = look_node(look);
        if (image_node.image.id(), image_node.color, image_node.rect)
            != (shown.image.id(), shown.color, shown.rect)
        {
            *image_node = shown;
        }
    }
    for (side, rect, look) in wanted {
        let mut node = Node::default();
        place(&mut node, rect);
        commands.spawn((
            Name::new("display pillar fill"),
            PillarFillNode { side },
            node,
            look_node(look),
            // Under the side panels, the toasts and every other node.
            GlobalZIndex(-1),
        ));
    }
}

fn look_node(look: Look) -> ImageNode {
    match look {
        Look::Solid(colour) => ImageNode::solid_color(colour),
        Look::Image { image, rect } => ImageNode {
            rect,
            ..ImageNode::new(image)
        },
    }
}

/// The ribbon image for `ribbon`, built once per bar size, side and
/// prescale. With `factors` other than one (sharp-bilinear) it is expanded
/// by them with nearest-neighbour and shown with a linear sampler, as the
/// `[prescale]` step shows the frame; otherwise it is sampled like the
/// faithful frame, nearest.
fn ribbon_image(
    state: &mut FillState,
    images: &mut Assets<Image>,
    font: &FixedCellFont,
    ribbon: RibbonBar,
    factors: UVec2,
) -> Handle<Image> {
    if let Some((.., handle)) = state
        .ribbons
        .iter()
        .find(|(built, scaled, _)| *built == ribbon && *scaled == factors)
    {
        return handle.clone();
    }
    let rgba = paint_ribbon_bar(font, ribbon, ChromePalette::EGA);
    let image = if factors == UVec2::ONE {
        let mut image = rgba_image(ribbon.size, rgba);
        image.sampler = ImageSampler::nearest();
        image
    } else {
        let mut expanded = Vec::new();
        prescale_nearest(&rgba, ribbon.size, factors, &mut expanded);
        let mut image = rgba_image(ribbon.size * factors, expanded);
        image.sampler = ImageSampler::linear();
        image
    };
    let handle = images.add(image);
    state
        .ribbons
        .retain(|(built, ..)| built.side != ribbon.side);
    state.ribbons.push((ribbon, factors, handle.clone()));
    handle
}

/// The bezel's image, uploaded once per decode.
fn bezel_handle(
    state: &mut FillState,
    images: &mut Assets<Image>,
    bezel: &BezelImage,
) -> Handle<Image> {
    if let Some(handle) = &state.bezel_image {
        return handle.clone();
    }
    let mut image = rgba_image(UVec2::new(bezel.width, bezel.height), bezel.rgba.to_vec());
    // Artwork, scaled smoothly.
    image.sampler = ImageSampler::linear();
    let handle = images.add(image);
    state.bezel_image = Some(handle.clone());
    handle
}

fn rgba_image(size: UVec2, rgba: Vec<u8>) -> Image {
    Image::new(
        Extent3d {
            width: size.x,
            height: size.y,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        rgba,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
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

#[cfg(test)]
mod tests;

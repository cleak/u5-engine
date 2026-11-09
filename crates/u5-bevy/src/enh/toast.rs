//! Toasts and shell badges (Phase B design 1.3.7, 3.2.10).
//!
//! One toast service for every feature: [`Toasts::short`] (2 s, refusals
//! and confirmations) and [`Toasts::note`] (5 s, staleness and write
//! errors). A toast is one line, and the newest replaces the one showing.
//! Every text is a `&'static str` constant in its feature module, and a
//! toast never names anything the player has not found (1.4.7).
//!
//! The toast is drawn by a `bevy_ui` node with its own CPU-built image, in
//! the window margin when one exists and otherwise as a strip over the
//! frame's top row; the frame image is never written. It follows the frame
//! where the side-space arbiter (1.3.8) puts it.
//!
//! [`ShellBadges`] holds the status badges chapters 6 and 7 set (automatic
//! combat, a modified game); the margin badge reads it.
//!
//! None of this is in the app until a feature needs it
//! ([`install_shell_services`]): with every such feature off no toast or
//! badge resource, node or system exists.

use std::time::Duration;

use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use u5_runtime::{CH_CELL_SIDE, FixedCellFont};

use super::EnhSet;
use super::side::{PanelSide, SideLayout};
use crate::{VISUAL_PLAY_FRAME_HEIGHT, VISUAL_PLAY_FRAME_WIDTH, VisualState, fitted_display_size};

/// How long a refusal or confirmation shows.
pub const SHORT_TOAST: Duration = Duration::from_secs(2);
/// How long a note (staleness, a write error) shows.
pub const NOTE_TOAST: Duration = Duration::from_secs(5);

/// The text grid of the 320x200 frame: one toast row is one of its rows.
const FRAME_COLUMNS: u32 = VISUAL_PLAY_FRAME_WIDTH / CH_CELL_SIDE as u32;
const FRAME_ROWS: u32 = VISUAL_PLAY_FRAME_HEIGHT / CH_CELL_SIDE as u32;
/// Light grey on black, the text window's own pens.
const TOAST_INK: [u8; 4] = [0xaa, 0xaa, 0xaa, 0xff];
const TOAST_PAPER: [u8; 4] = [0x00, 0x00, 0x00, 0xff];

/// How long a toast shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToastKind {
    /// A refusal or a confirmation: 2 s.
    Short,
    /// A note such as a write error: 5 s.
    Note,
}

impl ToastKind {
    pub const fn duration(self) -> Duration {
        match self {
            ToastKind::Short => SHORT_TOAST,
            ToastKind::Note => NOTE_TOAST,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Toast {
    text: &'static str,
    kind: ToastKind,
    /// Real time shown so far, counted from the frame after it was posted.
    shown: Duration,
    /// Posted this frame: the first tick does not age it.
    fresh: bool,
}

/// The one toast line. Features post to it; `tick_toasts` ages it in real
/// time, so an open Live overlay or a paused clock does not hold it.
#[derive(Resource, Debug, Default)]
pub struct Toasts {
    current: Option<Toast>,
}

impl Toasts {
    /// Shows `text` for 2 s, replacing any toast that shows.
    pub fn short(&mut self, text: &'static str) {
        self.post(text, ToastKind::Short);
    }

    /// Shows `text` for 5 s, replacing any toast that shows.
    pub fn note(&mut self, text: &'static str) {
        self.post(text, ToastKind::Note);
    }

    /// Shows `text` for `kind`'s time, replacing any toast that shows. The
    /// same text posted again starts its time again.
    pub fn post(&mut self, text: &'static str, kind: ToastKind) {
        self.current = Some(Toast {
            text,
            kind,
            shown: Duration::ZERO,
            fresh: true,
        });
    }

    /// The text that shows, if any.
    pub fn current(&self) -> Option<&'static str> {
        self.current.map(|toast| toast.text)
    }

    /// The kind of the toast that shows, if any.
    pub fn kind(&self) -> Option<ToastKind> {
        self.current.map(|toast| toast.kind)
    }

    /// Ages the toast by one frame's real time and drops it when its time
    /// is up. The frame it was posted in does not count, so a 2 s toast
    /// shows on every frame that starts less than 2 s after that one.
    pub fn tick(&mut self, delta: Duration) {
        let Some(toast) = self.current.as_mut() else {
            return;
        };
        if std::mem::take(&mut toast.fresh) {
            return;
        }
        toast.shown = toast.shown.saturating_add(delta);
        if toast.shown >= toast.kind.duration() {
            self.current = None;
        }
    }
}

/// The status badges. Chapter 6 sets `auto` while automatic combat drives
/// the party; chapter 7 sets `modified` once a cheat has changed the game.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ShellBadges {
    pub auto: bool,
    pub modified: bool,
}

impl ShellBadges {
    /// Whether any badge shows.
    pub fn any(self) -> bool {
        self.auto || self.modified
    }
}

/// Adds the toast line, the badges and the toast systems, once. Every
/// feature that posts a toast or sets a badge calls it from its own
/// install; nothing else does, so with those features off none of it
/// exists.
pub fn install_shell_services(app: &mut App) {
    if app.world().contains_resource::<Toasts>() {
        return;
    }
    app.init_resource::<Toasts>()
        .init_resource::<ShellBadges>()
        .add_systems(
            Update,
            (tick_toasts, draw_toast).chain().in_set(EnhSet::Present),
        );
}

/// Ages the toast in real time, in `EnhSet::Present`, before it is drawn.
fn tick_toasts(time: Res<Time<Real>>, mut toasts: ResMut<Toasts>) {
    toasts.tick(time.delta());
}

/// The toast strip's node, and the text its image shows.
#[derive(Component, Debug)]
pub struct ToastStrip {
    text: &'static str,
}

/// Where the strip sits, in logical window pixels. One text row of the
/// frame high, in the frame's cell size, and in the window margin when one
/// exists (1.3.7):
///
/// 1. the band above the frame, when a row fits there, centred on the
///    frame;
/// 2. else a bar beside the frame that holds no side panel, when the whole
///    line fits there, level with the frame's top;
/// 3. else over the frame's top row, centred on it.
///
/// A line wider than the frame shrinks to the frame's width. The frame is
/// where [`SideLayout`] puts it while it owns the frame, otherwise the
/// fitted 4:3 frame in the middle of the window.
pub(crate) fn toast_strip_rect(window: Vec2, layout: Option<&SideLayout>, cells: usize) -> Rect {
    let frame = layout.and_then(|layout| layout.frame).unwrap_or_else(|| {
        let size = fitted_display_size(VISUAL_PLAY_FRAME_WIDTH, VISUAL_PLAY_FRAME_HEIGHT, window);
        Rect::from_center_size(window / 2.0, size)
    });
    let cell = Vec2::new(
        frame.width() / FRAME_COLUMNS as f32,
        frame.height() / FRAME_ROWS as f32,
    );
    let natural = Vec2::new(cell.x * cells.max(1) as f32, cell.y);
    let size = natural * (frame.width() / natural.x).min(1.0);
    let at = |top_left: Vec2| Rect::from_corners(top_left, top_left + size);
    if frame.min.y >= size.y {
        return at(Vec2::new(
            frame.center().x - size.x / 2.0,
            frame.min.y - size.y,
        ));
    }
    let granted = |side: PanelSide| {
        layout.is_some_and(|layout| {
            layout
                .granted
                .iter()
                .any(|(kind, _)| layout.side_of(*kind) == Some(side))
        })
    };
    let bars = [
        (PanelSide::Left, 0.0, frame.min.x),
        (PanelSide::Right, frame.max.x, window.x),
    ];
    for (side, from, to) in bars {
        if !granted(side) && to - from >= natural.x {
            return at(Vec2::new((from + to - size.x) / 2.0, frame.min.y));
        }
    }
    at(Vec2::new(frame.center().x - size.x / 2.0, frame.min.y))
}

/// The strip's pixels: `text` in the frame's own font, one cell per byte,
/// light grey on black. A byte the font has no glyph for draws blank.
pub(crate) fn toast_strip_rgba(font: &FixedCellFont, text: &str) -> (u32, u32, Vec<u8>) {
    let cells = text.len().max(1);
    let width = cells * CH_CELL_SIDE;
    let mut rgba = vec![0; width * CH_CELL_SIDE * 4];
    for (cell, byte) in text
        .bytes()
        .chain(std::iter::repeat(b' '))
        .take(cells)
        .enumerate()
    {
        for row in 0..CH_CELL_SIDE {
            let bits = font.glyph_row(byte, row).unwrap_or(0);
            for column in 0..CH_CELL_SIDE {
                let pen = if bits & (0x80 >> column) != 0 {
                    TOAST_INK
                } else {
                    TOAST_PAPER
                };
                let offset = (row * width + cell * CH_CELL_SIDE + column) * 4;
                rgba[offset..offset + 4].copy_from_slice(&pen);
            }
        }
    }
    (width as u32, CH_CELL_SIDE as u32, rgba)
}

fn strip_image(font: &FixedCellFont, text: &str) -> Image {
    let (width, height, rgba) = toast_strip_rgba(font, text);
    let mut image = Image::new(
        Extent3d {
            width,
            height,
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
    node.position_type = PositionType::Absolute;
    node.left = Val::Px(rect.min.x);
    node.top = Val::Px(rect.min.y);
    node.width = Val::Px(rect.width());
    node.height = Val::Px(rect.height());
}

/// Shows the toast as a UI node in the margin or over the frame, in
/// `EnhSet::Present`. It builds the strip's image when the text changes and
/// follows the window size every frame. It reads the play frame's font, so
/// nothing draws before gameplay; the title layer brings its own font with
/// chapter 3.
fn draw_toast(
    mut commands: Commands,
    toasts: Res<Toasts>,
    visual: Option<Res<VisualState>>,
    layout: Option<Res<SideLayout>>,
    windows: Query<&Window>,
    mut images: ResMut<Assets<Image>>,
    mut strips: Query<(Entity, &ToastStrip, &mut Node)>,
) {
    let layout = layout.as_deref();
    let showing = toasts.current();
    let window = windows
        .single()
        .ok()
        .map(|window| Vec2::new(window.resolution.width(), window.resolution.height()));
    let mut drawn = false;
    for (entity, strip, mut node) in &mut strips {
        match (showing, window) {
            (Some(text), Some(window)) if text == strip.text && !drawn => {
                place(&mut node, toast_strip_rect(window, layout, text.len()));
                drawn = true;
            }
            _ => commands.entity(entity).despawn(),
        }
    }
    if drawn {
        return;
    }
    let (Some(text), Some(window), Some(visual)) = (showing, window, visual) else {
        return;
    };
    let image = images.add(strip_image(&visual.text_font, text));
    let mut node = Node::default();
    place(&mut node, toast_strip_rect(window, layout, text.len()));
    commands.spawn((
        Name::new("enh toast"),
        ToastStrip { text },
        node,
        ImageNode::new(image),
        GlobalZIndex(i32::MAX - 1),
    ));
}

#[cfg(test)]
mod tests;

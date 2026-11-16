//! Toasts and shell badges (Phase B design 1.3.7, 3.2.10).
//!
//! One toast service for every feature: [`Toasts::short`] (2 s, refusals
//! and confirmations) and [`Toasts::note`] (5 s, staleness and write
//! errors). A toast is one line, and the newest replaces the one showing.
//! Every text is a `&'static str` constant in its feature module, and a
//! toast never names anything the player has not found (1.4.7). A toast
//! that carries a name ([`Toasts::short_text`], [`Toasts::note_text`],
//! D-M4.7) is built with `format!` from such a constant; only a slot name
//! the player typed or the OS's own error text is ever put into one.
//!
//! The toast and the badges are `bevy_ui` nodes with their own CPU-built
//! images; the frame image is never written. They sit in the window margin
//! when one holds them (a side bar the side-space arbiter (1.3.8) leaves
//! free, else the bands above and below the frame), at a whole multiple of
//! the font's pixels, the toast wrapped at spaces; with no margin they sit
//! over the map, one cell in from its rules, clear of the grow-left
//! column, ribbon, tabs and crawl ([`shell_placement`]). Nothing is ever
//! drawn over the frame's border.
//!
//! [`ShellBadges`] holds the status badges chapters 6 and 7 set (automatic
//! combat, a modified game), drawn as label tabs in the frame's own chrome
//! (IBM.CH between ribbon end caps on a blue band): `AUTO` and `MOD`.
//!
//! None of this is in the app until a feature needs it
//! ([`install_shell_services`]): with every such feature off no toast or
//! badge resource, node or system exists.

use std::borrow::Cow;
use std::time::Duration;

use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use u5_runtime::{CH_CELL_SIDE, FixedCellFont};

use super::EnhSet;
use super::present::PresentHost;
use super::side::{PanelSide, SideLayout};
use crate::{VISUAL_PLAY_FRAME_HEIGHT, VISUAL_PLAY_FRAME_WIDTH, VisualState, fitted_display_size};

/// How long a refusal or confirmation shows.
pub const SHORT_TOAST: Duration = Duration::from_secs(2);
/// How long a note (staleness, a write error) shows.
pub const NOTE_TOAST: Duration = Duration::from_secs(5);

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

#[derive(Clone, Debug, PartialEq, Eq)]
struct Toast {
    text: Cow<'static, str>,
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

    /// Shows `text` for 2 s, replacing any toast that shows: a
    /// confirmation that carries a name, such as a slot's (D-M4.7). `text`
    /// is a `&'static str` template of the feature's text module filled
    /// with `format!`; only a slot name the player typed or an OS error is
    /// ever put into one.
    pub fn short_text(&mut self, text: String) {
        self.post_text(Cow::Owned(text), ToastKind::Short);
    }

    /// Shows `text` for 5 s, replacing any toast that shows: a note that
    /// carries a name or an OS reason. See [`Toasts::short_text`].
    pub fn note_text(&mut self, text: String) {
        self.post_text(Cow::Owned(text), ToastKind::Note);
    }

    /// Shows `text` for `kind`'s time, replacing any toast that shows. The
    /// same text posted again starts its time again.
    pub fn post(&mut self, text: &'static str, kind: ToastKind) {
        self.post_text(Cow::Borrowed(text), kind);
    }

    fn post_text(&mut self, text: Cow<'static, str>, kind: ToastKind) {
        self.current = Some(Toast {
            text,
            kind,
            shown: Duration::ZERO,
            fresh: true,
        });
    }

    /// The text that shows, if any.
    pub fn current(&self) -> Option<&str> {
        self.current.as_ref().map(|toast| toast.text.as_ref())
    }

    /// The kind of the toast that shows, if any.
    pub fn kind(&self) -> Option<ToastKind> {
        self.current.as_ref().map(|toast| toast.kind)
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

/// The toast's node, and the text its image shows.
#[derive(Component, Debug)]
pub struct ToastStrip {
    text: String,
    /// The lines the text was wrapped into for the place it shows in.
    lines: Vec<String>,
}

/// The badges' node, and the badges its image shows.
#[derive(Component, Debug)]
pub struct BadgeStrip {
    badges: ShellBadges,
}

/// The badge text (6.2.7, 7.4): `AUTO` while auto-combat drives anyone,
/// `MOD` once a cheat has changed the game.
pub const BADGE_AUTO: &str = "AUTO";
/// See [`BADGE_AUTO`].
pub const BADGE_MODIFIED: &str = "MOD";

/// The badges' z: under every overlay page (the cheats page, the menu, the
/// gem map), which they must never cover; the toast is the top node.
const BADGE_Z: i32 = i32::MAX - 5;
/// The toast's z: the top node.
const TOAST_Z: i32 = i32::MAX - 1;

/// The most lines a toast wraps into in a side bar, and over the map.
const BAR_LINES: usize = 3;
const MAP_LINES: usize = 4;

/// The badges `badges` shows, in order.
pub(crate) fn badge_words(badges: ShellBadges) -> Vec<&'static str> {
    let mut words = Vec::new();
    if badges.auto {
        words.push(BADGE_AUTO);
    }
    if badges.modified {
        words.push(BADGE_MODIFIED);
    }
    words
}

/// The badge strip's width in cells: a blue band cell at each end and
/// between the tabs, each tab its text closed by the two ribbon caps.
pub(crate) fn badge_cells(words: &[&str]) -> u32 {
    if words.is_empty() {
        return 0;
    }
    words
        .iter()
        .map(|word| crate::menu::frame::tab_cells(word.len()))
        .sum::<u32>()
        + words.len() as u32
        + 1
}

/// `text` wrapped at spaces into lines of at most `width` cells, or `None`
/// when a word is longer than a line.
pub(crate) fn wrap_words(text: &str, width: usize) -> Option<Vec<String>> {
    let mut lines: Vec<String> = Vec::new();
    for word in text.split(' ').filter(|word| !word.is_empty()) {
        if word.len() > width {
            return None;
        }
        match lines.last_mut() {
            Some(line) if line.len() + 1 + word.len() <= width => {
                line.push(' ');
                line.push_str(word);
            }
            _ => lines.push(word.to_string()),
        }
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    Some(lines)
}

/// Where the shell's toast and badges show, in logical window pixels
/// (design 1.3.7, 6.2.7, 7.4; never over the frame's rules, the ribbon,
/// the tabs or the crawl).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ShellPlacement {
    /// Window pixels per font pixel, across and down.
    pub glyph: Vec2,
    /// The toast's lines and the rectangle they fill.
    pub toast: Option<(Vec<String>, Rect)>,
    /// The badge strip's rectangle.
    pub badges: Option<Rect>,
}

/// Places the toast and the badges, in this order of preference:
///
/// 1. **A side bar** with no side panel in it (the right one first): the
///    badges at its top, a cell in from the frame and a cell down from the
///    frame's top; the toast at its bottom, a cell up from the frame's
///    bottom, wrapped at spaces into at most three lines. The font is drawn
///    at a whole multiple of its pixels, the largest from the picture's
///    own scale down to half of it that fits.
/// 2. **The bands** above and below the frame, when one holds a line and a
///    cell's gutter each side at such a scale: the badges right-aligned on
///    the frame in the top band, the toast right-aligned in the bottom one.
/// 3. **No margin** (a window exactly the frame's shape, or the grow-left
///    canvas, which fills the window): over the map, in the picture's own
///    cells, one cell in from its rules: the badges at its top right, the
///    toast at its bottom right, right of the grow-left column and crawl.
///
/// `picture` is the presented sprite's rectangle and its image's size (the
/// 320x200 frame or a grow-left canvas); without one, the fitted frame.
pub(crate) fn shell_placement(
    window: Vec2,
    layout: Option<&SideLayout>,
    picture: Option<(Rect, UVec2)>,
    toast: Option<&str>,
    badge_cells: u32,
) -> ShellPlacement {
    let cell = CH_CELL_SIDE as f32;
    let (frame, size) = match picture {
        Some((rect, size)) if size.x >= CH_CELL_SIDE as u32 && size.y >= CH_CELL_SIDE as u32 => {
            (rect, size)
        }
        _ => {
            let frame = layout.and_then(|layout| layout.frame).unwrap_or_else(|| {
                let size =
                    fitted_display_size(VISUAL_PLAY_FRAME_WIDTH, VISUAL_PLAY_FRAME_HEIGHT, window);
                Rect::from_center_size(window / 2.0, size)
            });
            (
                frame,
                UVec2::new(VISUAL_PLAY_FRAME_WIDTH, VISUAL_PLAY_FRAME_HEIGHT),
            )
        }
    };
    // The picture's own pixel, and its shape (1.2 tall on the DOS aspect).
    let pixel = Vec2::new(
        frame.width() / size.x as f32,
        frame.height() / size.y as f32,
    );
    let tall = pixel.y / pixel.x;
    let largest = pixel.x.floor().max(1.0) as u32;
    let smallest = (largest / 2).max(1);
    let badges_w = |glyph: Vec2| badge_cells as f32 * cell * glyph.x;

    // 1. A side bar without a panel.
    let granted = |side: PanelSide| {
        layout.is_some_and(|layout| {
            layout
                .granted
                .iter()
                .any(|(kind, _)| layout.side_of(*kind) == Some(side))
        })
    };
    let bars = [
        (PanelSide::Right, frame.max.x, window.x),
        (PanelSide::Left, 0.0, frame.min.x),
    ];
    for (side, from, to) in bars {
        if granted(side) {
            continue;
        }
        for scale in (smallest..=largest).rev() {
            let glyph = Vec2::new(scale as f32, scale as f32 * tall);
            let gutter = cell * glyph.x;
            let room = to - from - 2.0 * gutter;
            if room < cell * glyph.x || badges_w(glyph) > room {
                continue;
            }
            let lines = match toast {
                Some(text) => match wrap_words(text, (room / (cell * glyph.x)).floor() as usize) {
                    Some(lines) if lines.len() <= BAR_LINES => Some(lines),
                    _ => continue,
                },
                None => None,
            };
            // The edge nearer the frame.
            let x = if side == PanelSide::Right {
                from + gutter
            } else {
                to - gutter
            };
            let line_h = cell * glyph.y;
            let toast = lines.map(|lines| {
                let w = cells_of(&lines) as f32 * cell * glyph.x;
                let h = lines.len() as f32 * line_h;
                let x0 = if side == PanelSide::Right { x } else { x - w };
                let rect = Rect::new(x0, frame.max.y - line_h - h, x0 + w, frame.max.y - line_h);
                (lines, rect)
            });
            let badges = (badge_cells > 0).then(|| {
                let w = badges_w(glyph);
                let x0 = if side == PanelSide::Right { x } else { x - w };
                let y0 = frame.min.y + line_h;
                Rect::new(x0, y0, x0 + w, y0 + line_h)
            });
            if toast
                .as_ref()
                .zip(badges)
                .is_some_and(|((_, t), b)| t.min.y < b.max.y)
            {
                continue;
            }
            return ShellPlacement {
                glyph,
                toast,
                badges,
            };
        }
    }

    // 2. The bands above and below the frame.
    let band = frame.min.y.min(window.y - frame.max.y);
    for scale in (smallest..=largest).rev() {
        let glyph = Vec2::new(scale as f32, scale as f32 * tall);
        let line_h = cell * glyph.y;
        let gutter = cell * glyph.x;
        if band < 3.0 * line_h {
            continue;
        }
        let width = frame.width() - 2.0 * gutter;
        let max_lines = ((band - 2.0 * line_h) / line_h).floor().max(1.0) as usize;
        let lines = match toast {
            Some(text) => match wrap_words(text, (width / (cell * glyph.x)).floor() as usize) {
                Some(lines) if lines.len() <= max_lines => Some(lines),
                _ => continue,
            },
            None => None,
        };
        if badges_w(glyph) > width {
            continue;
        }
        let right = frame.max.x - gutter;
        let toast = lines.map(|lines| {
            let w = cells_of(&lines) as f32 * cell * glyph.x;
            let h = lines.len() as f32 * line_h;
            let y0 = frame.max.y + (band - h) / 2.0;
            (lines, Rect::new(right - w, y0, right, y0 + h))
        });
        let badges = (badge_cells > 0).then(|| {
            let y0 = (frame.min.y - line_h) / 2.0;
            Rect::new(right - badges_w(glyph), y0, right, y0 + line_h)
        });
        return ShellPlacement {
            glyph,
            toast,
            badges,
        };
    }

    // 3. No margin: over the map, in the picture's own cells.
    let glyph = pixel;
    let (cols, rows) = (size.x / CH_CELL_SIDE as u32, size.y / CH_CELL_SIDE as u32);
    let classic = size == UVec2::new(VISUAL_PLAY_FRAME_WIDTH, VISUAL_PLAY_FRAME_HEIGHT);
    // The map's cells inside its rules, then one cell in from them.
    let (left, top, right, bottom) = if classic {
        (1, 1, MAP_CELLS, MAP_CELLS)
    } else {
        // The grow-left map window (8-pixel sides, a 16-row top band and an
        // 8-row bottom one), right of the column's rule and the crawl.
        (
            CANVAS_CLEAR_COL.min(cols.saturating_sub(2)),
            2,
            cols.saturating_sub(2),
            rows.saturating_sub(2),
        )
    };
    let (left, top, right, bottom) = (
        left + 1,
        top + 1,
        right.saturating_sub(1),
        bottom.saturating_sub(1),
    );
    let at = |col: u32, row: u32| {
        Vec2::new(
            frame.min.x + col as f32 * cell * glyph.x,
            frame.min.y + row as f32 * cell * glyph.y,
        )
    };
    let width = (right + 1).saturating_sub(left) as usize;
    let toast = toast.map(|text| {
        let lines = wrap_words(text, width)
            .filter(|lines| lines.len() <= MAP_LINES)
            .unwrap_or_else(|| vec![text.chars().take(width).collect()]);
        let w = cells_of(&lines);
        let first = (bottom + 1).saturating_sub(lines.len() as u32);
        let min = at(right + 1 - w.min(right + 1), first);
        let max = at(right + 1, bottom + 1);
        (lines, Rect::from_corners(min, max))
    });
    let badges = (badge_cells > 0).then(|| {
        let min = at((right + 1).saturating_sub(badge_cells), top);
        Rect::from_corners(min, at(right + 1, top + 1))
    });
    ShellPlacement {
        glyph,
        toast,
        badges,
    }
}

/// [`shell_placement`] for the toast and the badges each on its own, so
/// one that fits a margin goes there even when the other does not; both
/// together when the two would overlap there.
pub(crate) fn shell_placements(
    window: Vec2,
    layout: Option<&SideLayout>,
    picture: Option<(Rect, UVec2)>,
    toast: Option<&str>,
    badge_cells: u32,
) -> ShellPlacement {
    let alone = shell_placement(window, layout, picture, toast, 0);
    let badges = shell_placement(window, layout, picture, None, badge_cells);
    let overlap = alone
        .toast
        .as_ref()
        .zip(badges.badges)
        .is_some_and(|((_, toast), badges)| !toast.intersect(badges).is_empty());
    if overlap {
        return shell_placement(window, layout, picture, toast, badge_cells);
    }
    ShellPlacement {
        glyph: alone.glyph,
        toast: alone.toast,
        badges: badges.badges,
    }
}

/// The classic frame's map: cells 1-22 inside its rules.
const MAP_CELLS: u32 = 22;
/// On a grow-left canvas, the first cell right of the open column's rule
/// (x 151) and of the crawl's lines.
const CANVAS_CLEAR_COL: u32 = 19;

fn cells_of(lines: &[String]) -> u32 {
    lines.iter().map(String::len).max().unwrap_or(1).max(1) as u32
}

/// The toast's pixels: `lines` in the frame's own font, one cell per byte,
/// light grey on black, each line padded to the longest. A byte the font
/// has no glyph for draws blank.
pub(crate) fn toast_rgba(font: &FixedCellFont, lines: &[String]) -> (u32, u32, Vec<u8>) {
    let cells = cells_of(lines) as usize;
    let width = cells * CH_CELL_SIDE;
    let height = lines.len().max(1) * CH_CELL_SIDE;
    let mut rgba = vec![0; width * height * 4];
    for (line_index, line) in lines.iter().enumerate() {
        for (cell, byte) in line
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
                    let y = line_index * CH_CELL_SIDE + row;
                    let offset = (y * width + cell * CH_CELL_SIDE + column) * 4;
                    rgba[offset..offset + 4].copy_from_slice(&pen);
                }
            }
        }
    }
    (width as u32, height as u32, rgba)
}

/// One line of toast pixels (the M4-01 sheet's strip).
#[cfg(test)]
pub(crate) fn toast_strip_rgba(font: &FixedCellFont, text: &str) -> (u32, u32, Vec<u8>) {
    toast_rgba(font, &[text.to_string()])
}

/// The badges' pixels, in the approved chrome (the c04f look): a blue band
/// row (EGA 1) in which each badge is a label tab, its word in IBM.CH on
/// black between the two ribbon end caps, as `>Calm Winds<` sits on its
/// band. `AUTO` is white; `MOD` is the cheats' brown accent (EGA 6).
pub(crate) fn badge_rgba(font: &FixedCellFont, words: &[&str]) -> (u32, u32, Vec<u8>) {
    use crate::menu::frame::{ACCENT, BAND, INK, MenuImage, draw_label_tab, ega, tab_cells};
    let cells = badge_cells(words).max(1);
    let mut img = MenuImage::new(cells * CH_CELL_SIDE as u32, CH_CELL_SIDE as u32);
    img.fill((0, 0, img.w - 1, img.h - 1), ega(BAND));
    let mut col = 1;
    for word in words {
        let ink = if *word == BADGE_MODIFIED { ACCENT } else { INK };
        draw_label_tab(&mut img, font, col, 0, word, ink);
        col += tab_cells(word.len()) + 1;
    }
    (img.w, img.h, img.rgba)
}

fn rgba_image(width: u32, height: u32, rgba: Vec<u8>) -> Image {
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

/// The picture the window shows: the presented canvas or frame sprite's
/// rectangle in window pixels, and its image's size.
fn shown_picture(
    window: &Window,
    present: Option<&PresentHost>,
    visual: Option<&VisualState>,
    sprites: &Query<(&Sprite, &Transform)>,
    images: &Assets<Image>,
) -> Option<(Rect, UVec2)> {
    let presented = present.and_then(PresentHost::presented);
    let faithful = visual.map(|visual| &visual.image_handle);
    let (sprite, transform) = sprites
        .iter()
        .find(|(sprite, _)| Some(&sprite.image) == presented || Some(&sprite.image) == faithful)?;
    let size = sprite.custom_size? * transform.scale.truncate();
    let centre = Vec2::new(
        window.resolution.width() / 2.0 + transform.translation.x,
        window.resolution.height() / 2.0 - transform.translation.y,
    );
    let pixels = images.get(&sprite.image)?.size();
    Some((Rect::from_center_size(centre, size), pixels))
}

/// Shows the toast and the badges as UI nodes where [`shell_placement`]
/// puts them, in `EnhSet::Present`. Each image is built when its text
/// changes (or the toast's wrap does); the nodes follow the window every
/// frame. It reads the play frame's font, so nothing draws before
/// gameplay; the title layer brings its own font with chapter 3.
#[allow(clippy::too_many_arguments)]
fn draw_toast(
    mut commands: Commands,
    toasts: Res<Toasts>,
    badges: Res<ShellBadges>,
    visual: Option<Res<VisualState>>,
    layout: Option<Res<SideLayout>>,
    present: Option<Res<PresentHost>>,
    windows: Query<&Window>,
    sprites: Query<(&Sprite, &Transform)>,
    mut images: ResMut<Assets<Image>>,
    mut strips: Query<(Entity, &ToastStrip, &mut Node), Without<BadgeStrip>>,
    mut badge_nodes: Query<(Entity, &BadgeStrip, &mut Node), Without<ToastStrip>>,
) {
    let window = windows.single().ok();
    let picture = window.and_then(|window| {
        shown_picture(
            window,
            present.as_deref(),
            visual.as_deref(),
            &sprites,
            &images,
        )
    });
    let size =
        window.map(|window| Vec2::new(window.resolution.width(), window.resolution.height()));
    let showing = toasts.current();
    let words = badge_words(*badges);
    let placement = size.map(|size| {
        shell_placements(
            size,
            layout.as_deref(),
            picture,
            showing,
            badge_cells(&words),
        )
    });
    let font = visual.as_deref().map(|visual| &visual.text_font);

    // The toast.
    let want = placement
        .as_ref()
        .and_then(|placement| placement.toast.clone())
        .zip(showing);
    let mut drawn = false;
    for (entity, strip, mut node) in &mut strips {
        match &want {
            Some(((lines, rect), text))
                if *text == strip.text && *lines == strip.lines && !drawn =>
            {
                place(&mut node, *rect);
                drawn = true;
            }
            _ => commands.entity(entity).despawn(),
        }
    }
    if !drawn && let (Some(((lines, rect), text)), Some(font)) = (want, font) {
        let (w, h, rgba) = toast_rgba(font, &lines);
        let mut node = Node::default();
        place(&mut node, rect);
        commands.spawn((
            Name::new("enh toast"),
            ToastStrip {
                text: text.to_string(),
                lines,
            },
            node,
            ImageNode::new(images.add(rgba_image(w, h, rgba))),
            GlobalZIndex(TOAST_Z),
        ));
    }

    // The badges.
    let want = placement.and_then(|placement| placement.badges);
    let mut drawn = false;
    for (entity, strip, mut node) in &mut badge_nodes {
        match want {
            Some(rect) if strip.badges == *badges && !drawn => {
                place(&mut node, rect);
                drawn = true;
            }
            _ => commands.entity(entity).despawn(),
        }
    }
    if !drawn && let (Some(rect), Some(font)) = (want, font) {
        let (w, h, rgba) = badge_rgba(font, &words);
        let mut node = Node::default();
        place(&mut node, rect);
        commands.spawn((
            Name::new("enh badges"),
            BadgeStrip { badges: *badges },
            node,
            ImageNode::new(images.add(rgba_image(w, h, rgba))),
            GlobalZIndex(BADGE_Z),
        ));
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod review_tests;

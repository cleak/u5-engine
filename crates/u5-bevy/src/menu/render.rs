//! The menu on screen (Phase B design 3.3.4 "Rendering", 3.4.4 "Chapter
//! 2"; build plan M4-05, D-M4.1, D-M4.10).
//!
//! - [`render_page`] draws the open page's rows with the M4-01 frame kit
//!   ([`super::frame`]) into a picture the size of the one the frame sprite
//!   shows: the 320x200 frame on 4:3 and on fixed screens, the grow-left
//!   canvas otherwise ([`super::geometry::menu_picture`]). Panel pixels are
//!   opaque; everything else is clear.
//! - `menu_render` (`EnhSet::Present`) redraws that picture only when the
//!   page, its rows, the picture or the column's place changes, and
//!   recomposes the node's image only when the sweep step changes.
//! - `place_overlay` (`EnhSet::Present`, after the render) puts the menu's
//!   `ImageNode` ([`MenuNode`]) exactly over the frame sprite's rectangle,
//!   with layout rounding off, so one menu pixel is one game pixel at every
//!   scale, and a full-window backdrop node ([`MenuBackdrop`], the kit's 0.6 dim as
//!   the GPU blends it) under it, which dims the frame, the bars, the side fill and the
//!   side panels. The toasts (`i32::MAX - 1`) stay above both. A gem view
//!   is never the idle prompt, so the gem map is never up with the menu and
//!   the two share its layers ([`MENU_NODE_Z`], [`MENU_BACKDROP_Z`]). The
//!   menu is never a `Sprite`, so `fit_display_to_window` never sees it.
//!
//! The picture over the backdrop is the kit's dimmed picture over the
//! frame (`frame::composite_over` of `frame::dim_backdrop`): a clear pixel
//! shows the backdrop's dim, an opaque one covers it. The backdrop's
//! opacity ([`BACKDROP_ALPHA`]) is chosen for the GPU's linear-light blend,
//! so the real dim looks as the sheet's does.
//!
//! **The sweep** (D-M4.10) is the M1 dither law over the panel's rectangle
//! only ([`super::frame::panel_step`]), rising from step 1 on open and
//! falling from step 7 on close, a step every [`MENU_SWEEP_STEP`]. A close
//! that interrupts an open (or the reverse) turns back from the step it is
//! on. Keys are never held for it: a key during the sweep changes the model
//! that frame, the picture is redrawn and the sweep goes on. Page changes
//! are instant. A close because the game left the prompt
//! ([`CloseReason::NotIdle`]) skips it: the menu goes that frame.

use std::time::Duration;

use bevy::ecs::system::SystemParam;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::ui::LayoutConfig;
use u5_runtime::FixedCellFont;

use super::frame::Scroll;
use super::frame::*;
use super::geometry::{CellRect, MenuPicture, PageShape, menu_picture, panel_rect};
use super::pages::{Page, PageState, Row, RowKind, ShellSnapshot, page_rows};
use super::text as t;
use super::{MenuDoors, MenuShell, MenuState, snapshot};
use crate::VisualState;
use crate::display::Display;
use crate::display::effects::{STEP_SLACK, SWEEP_STEPS};
use crate::display::grow::column::{ColumnState, ColumnView, page_mode_active};
use crate::display::grow::{Canvas, grow_canvas};
use crate::enh::gem_map::view::{GEM_BEFORE_Z, GEM_NODE_Z};
use crate::enh::overlay::{CloseReason, OverlayChanged, OverlayId};
use crate::enh::present::PresentHost;
use crate::enh::side::FrameImageSize;

/// The menu node's layer: the gem map's, above the frame sprite, the side
/// fill and the side panels, below the toasts.
pub const MENU_NODE_Z: i32 = GEM_NODE_Z;
/// The backdrop's layer, directly under the menu node.
pub const MENU_BACKDROP_Z: i32 = GEM_BEFORE_Z;

/// The backdrop's opacity on the GPU. The kit's dim (and the M4-01 sheet)
/// is black at 0.6 blended on the sRGB values ([`composite_over`]); the
/// GPU blends UI nodes in linear light, where 0.6 dims far less (white to
/// 170, the band's blue to 112, against 102 and 68 on the sheet). Black at
/// this opacity in linear light lands within 4 of the sheet's dim on every
/// EGA level (pinned by `tests::the_gpu_backdrop_matches_the_sheet_s_dim`).
pub const BACKDROP_ALPHA: f32 = 0.858;

/// The backdrop's colour: black at [`BACKDROP_ALPHA`].
pub fn backdrop_colour() -> Color {
    Color::srgba(0.0, 0.0, 0.0, BACKDROP_ALPHA)
}

/// Rows of the top menu's footer box.
const TOP_FOOTER_ROWS: u32 = 3;
/// Rows of a Settings page's footer box.
const SETTINGS_FOOTER_ROWS: u32 = 4;
/// The cell a Settings row's value starts at: after the margin, the label
/// column and one blank.
pub const VALUE_COL: u32 = 1 + t::SETTINGS_LABEL as u32 + 1;

/// One page drawn: the picture (clear outside the panel) and the panel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Drawn {
    pub image: MenuImage,
    pub panel: CellRect,
}

/// The first row shown of a list of `len` rows with room for `visible`,
/// so that row `cursor` is in sight: the list scrolls only once the cursor
/// passes the last visible row.
pub fn first_shown(cursor: usize, len: usize, visible: usize) -> usize {
    if visible == 0 || len <= visible {
        return 0;
    }
    cursor
        .min(len - 1)
        .saturating_sub(visible - 1)
        .min(len - visible)
}

/// The rows `bx` shows of `rows`, below its margin row, from `first`;
/// their lines from `line`, the selected one as the selection bar, with
/// the scroll marks on the bands above (`upper`) and below (`lower`).
#[allow(clippy::too_many_arguments)]
fn draw_list(
    img: &mut MenuImage,
    font: &FixedCellFont,
    bx: CellRect,
    upper: u32,
    lower: u32,
    rows: &[Row],
    cursor: usize,
    line: impl Fn(&Row, u32) -> Line,
) {
    let visible = (bx.h - 1) as usize;
    let first = first_shown(cursor, rows.len(), visible);
    for (i, row) in rows.iter().enumerate().skip(first).take(visible) {
        let at = bx.row + 1 + (i - first) as u32;
        draw_row(img, font, bx, at, &line(row, bx.w), i == cursor);
    }
    if first > 0 {
        draw_scroll_mark(img, font, bx.right() - 1, upper, Scroll::Up);
    }
    if first + visible < rows.len() {
        draw_scroll_mark(img, font, bx.right() - 1, lower, Scroll::Down);
    }
}

/// `text` wrapped into the footer box `bx` in the help ink, "Spoiler:" in
/// brown; lines past the box are dropped.
fn draw_help(img: &mut MenuImage, font: &FixedCellFont, bx: CellRect, text: &str) {
    for (i, line) in t::wrap(text, (bx.w - 2) as usize)
        .iter()
        .take(bx.h as usize)
        .enumerate()
    {
        let mut l = Line::new(bx.w).text(1, line, Ink::Help);
        if let Some(at) = line.find(t::SPOILER) {
            l = l.text(1 + at as u32, t::SPOILER, Ink::Accent);
        }
        draw_row(img, font, bx, bx.row + i as u32, &l, false);
    }
}

/// A top-menu row: its label after the margin cell, grey when disabled.
pub fn top_line(row: &Row, w: u32) -> Line {
    let line = Line::new(w).text(1, row.label, Ink::Plain);
    if row.enabled { line } else { line.greyed() }
}

/// A Settings row: its label after the margin cell ("Spoiler:" in brown),
/// a check box at [`VALUE_COL`], grey when disabled. Values and apply tags
/// come with the Settings pages (M4-06, M4-07).
pub fn setting_line(row: &Row, w: u32) -> Line {
    let mut line = Line::new(w).text(1, row.label, Ink::Plain);
    if let Some(at) = row.label.find(t::SPOILER) {
        line = line.text(1 + at as u32, t::SPOILER, Ink::Accent);
    }
    if let RowKind::Check { on } = row.kind {
        line = line.check(VALUE_COL, on);
    }
    if row.enabled { line } else { line.greyed() }
}

/// The help the footer shows for the selected row: its reason when it is
/// greyed.
fn selected_reason(rows: &[Row], cursor: usize) -> Option<&'static str> {
    rows.get(cursor).and_then(|row| row.reason)
}

/// The top menu (3.2.1): the narrow panel, its rows, a footer box and the
/// "Menu" and "F12: close" tabs.
fn top_page(
    font: &FixedCellFont,
    picture: &MenuPicture,
    column: ColumnView,
    rows: &[Row],
    cursor: usize,
) -> Drawn {
    let rect = panel_rect(PageShape::Narrow, picture, column);
    let divider = rect.bottom() - TOP_FOOTER_ROWS - 1;
    let panel = Panel::new(rect).divider(divider);
    let mut img = MenuImage::for_picture(picture);
    draw_panel(&mut img, &panel);
    let boxes = panel.boxes();
    let (list, footer) = (boxes[0], boxes[1]);
    draw_list(
        &mut img, font, list, rect.row, divider, rows, cursor, top_line,
    );
    match selected_reason(rows, cursor) {
        Some(reason) => draw_help(&mut img, font, footer, reason),
        // On the footer's second row, as the sheet has it: the first is
        // the last-save line's (M4-10).
        None => {
            let waits = Line::new(footer.w).text(1, t::WAITING, Ink::Help);
            draw_row(&mut img, font, footer, footer.row + 1, &waits, false);
        }
    }
    draw_title_tab(&mut img, font, rect, t::MENU_TITLE);
    draw_footer_tab(&mut img, font, rect, t::KEY_CLOSE);
    Drawn {
        image: img,
        panel: rect,
    }
}

/// A Settings page (3.2.7): the wide panel with the tab strip, its rows, a
/// footer box for the selected row's help and the "Esc: back" tab.
fn settings_page(
    font: &FixedCellFont,
    picture: &MenuPicture,
    column: ColumnView,
    tab: usize,
    rows: &[Row],
    cursor: usize,
) -> Drawn {
    let rect = panel_rect(PageShape::Wide, picture, column);
    let lower = rect.bottom() - SETTINGS_FOOTER_ROWS - 1;
    let panel = Panel::new(rect).divider(lower);
    let mut img = MenuImage::for_picture(picture);
    draw_panel(&mut img, &panel);
    let boxes = panel.boxes();
    let (list, footer) = (boxes[0], boxes[1]);
    draw_list(
        &mut img,
        font,
        list,
        rect.row,
        lower,
        rows,
        cursor,
        setting_line,
    );
    if let Some(reason) = selected_reason(rows, cursor) {
        draw_help(&mut img, font, footer, reason);
    }
    draw_tab_strip(&mut img, font, rect, &t::SETTINGS_TABS, tab);
    draw_footer_tab(&mut img, font, rect, t::KEY_BACK);
    Drawn {
        image: img,
        panel: rect,
    }
}

/// The page `state` of the menu for the shell in `snap`, drawn on
/// `picture` with the docked column in `column`'s state.
pub fn render_page(
    font: &FixedCellFont,
    picture: &MenuPicture,
    column: ColumnView,
    state: PageState,
    snap: &ShellSnapshot,
) -> Drawn {
    let rows = page_rows(state.page, snap);
    let cursor = state.cursor.min(rows.len().saturating_sub(1));
    match state.page {
        Page::Top => top_page(font, picture, column, &rows, cursor),
        Page::Settings(tab) => settings_page(font, picture, column, tab.index(), &rows, cursor),
    }
}

/// The sweep's state (D-M4.10): where it is going and the step it shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MenuSweep {
    /// True while the menu is (or is becoming) shown.
    pub open: bool,
    /// The step drawn, 1..=7, while a sweep runs; `None` when it is over:
    /// the whole menu while `open`, nothing otherwise.
    pub step: Option<u8>,
    elapsed: Duration,
}

impl MenuSweep {
    /// The menu opened: in from step 1, or back from the step a close is on.
    pub fn start_open(&mut self) {
        self.open = true;
        self.step = Some(self.step.unwrap_or(1));
        self.elapsed = Duration::ZERO;
    }

    /// The menu closed: out from step 7, or back from the step an open is
    /// on.
    pub fn start_close(&mut self) {
        self.open = false;
        self.step = Some(self.step.unwrap_or(SWEEP_STEPS - 1));
        self.elapsed = Duration::ZERO;
    }

    /// Gone at once.
    pub fn stop(&mut self) {
        *self = Self::default();
    }

    /// Whether anything of the menu shows.
    pub fn visible(&self) -> bool {
        self.open || self.step.is_some()
    }

    /// `dt` of shell frame time passed: a step every [`MENU_SWEEP_STEP`],
    /// towards 8 (the whole menu) when opening and 0 (none) when closing.
    pub fn tick(&mut self, dt: Duration) {
        if self.step.is_none() {
            self.elapsed = Duration::ZERO;
            return;
        }
        self.elapsed += dt;
        while let Some(k) = self.step
            && self.elapsed + STEP_SLACK >= MENU_SWEEP_STEP
        {
            self.elapsed = self.elapsed.saturating_sub(MENU_SWEEP_STEP);
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

/// What a drawn page was drawn from; a new one redraws it.
#[derive(Clone, Debug, PartialEq, Eq)]
struct DrawKey {
    state: PageState,
    rows: Vec<Row>,
    picture: MenuPicture,
    column_shown: bool,
}

/// What the menu shows and how far its sweep is. Exists while the menu
/// plugin is in the app.
#[derive(Resource, Debug, Default)]
pub struct MenuView {
    pub sweep: MenuSweep,
    /// The page last drawn, kept through the close sweep.
    drawn: Option<(DrawKey, Drawn)>,
    /// The node's picture and the draw and step it was composed from.
    image: Option<Handle<Image>>,
    composed: Option<(u64, Option<u8>)>,
    /// How many times a page was drawn, and the node's picture composed.
    pub renders: u64,
    pub composes: u64,
}

impl MenuView {
    /// The node's picture, while the menu shows.
    pub fn image(&self) -> Option<&Handle<Image>> {
        self.image.as_ref().filter(|_| self.sweep.visible())
    }

    /// The page last drawn.
    pub fn drawn(&self) -> Option<&Drawn> {
        self.drawn.as_ref().map(|(_, drawn)| drawn)
    }

    /// The picture's size, while the menu shows.
    pub fn picture_size(&self) -> Option<UVec2> {
        self.drawn()
            .filter(|_| self.sweep.visible())
            .map(|drawn| UVec2::new(drawn.image.w, drawn.image.h))
    }
}

/// The shell state the picture's size and the panel's place come from.
#[derive(SystemParam)]
pub(crate) struct PictureInputs<'w, 's> {
    visual: Option<Res<'w, VisualState>>,
    display: Option<Res<'w, Display>>,
    frame_size: Option<Res<'w, FrameImageSize>>,
    column: Option<Res<'w, ColumnState>>,
    windows: Query<'w, 's, &'static Window>,
}

impl PictureInputs<'_, '_> {
    /// The grow-left canvas the frame sprite shows this frame, if any: the
    /// one the frame fit placed (`FrameImageSize`).
    fn canvas(&self) -> Option<Canvas> {
        let size = self.frame_size.as_deref()?.0?;
        let display = self.display.as_deref()?;
        let window = self.windows.single().ok()?.physical_size();
        let state = self.visual.as_deref().map(|visual| &visual.state);
        let canvas = grow_canvas(state, window, display.0.pixel_aspect)?;
        (UVec2::new(canvas.w, canvas.h) == size).then_some(canvas)
    }

    /// The picture and the column's state.
    pub(crate) fn picture(&self) -> (MenuPicture, ColumnView) {
        let mut column = self
            .column
            .as_deref()
            .map(ColumnState::view)
            .unwrap_or_default();
        if let Some(visual) = self.visual.as_deref() {
            column.page_mode = page_mode_active(&visual.state);
        }
        (menu_picture(self.canvas().as_ref()), column)
    }
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

/// Writes `img` into the image `handle` names when it has the same size,
/// else into a new image; returns the handle in use.
fn upload(
    images: &mut Assets<Image>,
    handle: Option<Handle<Image>>,
    img: &MenuImage,
) -> Handle<Image> {
    let size = UVec2::new(img.w, img.h);
    if let Some(handle) = handle {
        if let Some(image) = images.get_mut(&handle)
            && image.size() == size
        {
            image.data = Some(img.rgba.clone());
            return handle;
        }
        images.remove(&handle);
    }
    images.add(rgba_image(img.w, img.h, img.rgba.clone()))
}

/// Starts and steps the sweep, redraws the page when what it shows
/// changed and recomposes the node's picture when the draw or the step
/// changed.
#[allow(clippy::too_many_arguments)]
pub(crate) fn menu_render(
    mut changed: EventReader<OverlayChanged>,
    state: Res<MenuState>,
    shell: Res<MenuShell>,
    doors: Res<MenuDoors>,
    inputs: PictureInputs,
    time: Res<Time>,
    mut view: ResMut<MenuView>,
    mut images: ResMut<Assets<Image>>,
) {
    let view = &mut *view;
    let mut started = false;
    for change in changed.read() {
        match *change {
            OverlayChanged::Opened(OverlayId::Menu) => {
                view.sweep.start_open();
                started = true;
            }
            OverlayChanged::Closed {
                id: OverlayId::Menu,
                why,
            } => {
                if why == CloseReason::NotIdle {
                    view.sweep.stop();
                } else {
                    view.sweep.start_close();
                }
                started = true;
            }
            _ => {}
        }
    }
    if !started {
        view.sweep.tick(time.delta());
    }
    if let (Some(model), Some(visual)) = (state.model(), inputs.visual.as_deref()) {
        let snap = snapshot(&shell, &doors);
        let (picture, column) = inputs.picture();
        let current = model.current();
        let key = DrawKey {
            state: current,
            rows: page_rows(current.page, &snap),
            picture,
            column_shown: column.shown(),
        };
        if view.drawn.as_ref().map(|(drawn_from, _)| drawn_from) != Some(&key) {
            let drawn = render_page(&visual.text_font, &picture, column, current, &snap);
            view.drawn = Some((key, drawn));
            view.renders += 1;
        }
    }
    if !view.sweep.visible() || view.drawn.is_none() {
        view.sweep.stop();
        view.drawn = None;
        view.composed = None;
        if let Some(handle) = view.image.take() {
            images.remove(&handle);
        }
        return;
    }
    let composed = (view.renders, view.sweep.step);
    if view.composed == Some(composed) && view.image.is_some() {
        return;
    }
    let Some((_, drawn)) = view.drawn.as_ref() else {
        return;
    };
    let picture = sweep_image_over(&drawn.image, drawn.panel, view.sweep.step, CLEAR);
    let handle = view.image.take();
    view.image = Some(upload(&mut images, handle, &picture));
    view.composed = Some(composed);
    view.composes += 1;
}

/// The menu's picture over the frame sprite.
#[derive(Component, Debug)]
pub struct MenuNode;

/// The full-window dim under it.
#[derive(Component, Debug)]
pub struct MenuBackdrop;

/// The frame sprite's rectangle in window coordinates (logical pixels):
/// the 2D camera puts the world origin at the window centre, y up.
pub fn frame_rect(window: &Window, sprite: &Sprite, transform: &Transform) -> Option<Rect> {
    let size = sprite.custom_size? * transform.scale.truncate();
    let centre = Vec2::new(
        window.resolution.width() / 2.0 + transform.translation.x,
        window.resolution.height() / 2.0 - transform.translation.y,
    );
    Some(Rect::from_center_size(centre, size))
}

fn set_rect(node: &mut Node, rect: Rect) {
    let wanted = [
        Val::Px(rect.min.x),
        Val::Px(rect.min.y),
        Val::Px(rect.width()),
        Val::Px(rect.height()),
    ];
    for (field, value) in [
        &mut node.left,
        &mut node.top,
        &mut node.width,
        &mut node.height,
    ]
    .into_iter()
    .zip(wanted)
    {
        if *field != value {
            *field = value;
        }
    }
}

fn absolute(rect: Rect) -> Node {
    let mut node = Node {
        position_type: PositionType::Absolute,
        ..default()
    };
    set_rect(&mut node, rect);
    node
}

/// The full window: the backdrop's node.
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

/// Run condition: the menu shows, or one of its nodes is left to remove.
pub(crate) fn place_due(
    view: Res<MenuView>,
    nodes: Query<(), Or<(With<MenuNode>, With<MenuBackdrop>)>>,
) -> bool {
    view.image().is_some() || !nodes.is_empty()
}

/// Spawns, places and removes the menu node and the backdrop. The frame
/// sprite is the one showing the game (the presented or the faithful
/// image); the node takes its rectangle exactly.
#[allow(clippy::type_complexity)]
pub(crate) fn place_overlay(
    mut commands: Commands,
    view: Res<MenuView>,
    visual: Option<Res<VisualState>>,
    present: Option<Res<PresentHost>>,
    windows: Query<&Window>,
    sprites: Query<(&Sprite, &Transform)>,
    mut nodes: Query<(Entity, &mut Node, &mut ImageNode), With<MenuNode>>,
    backdrops: Query<Entity, With<MenuBackdrop>>,
) {
    let rect = windows.single().ok().and_then(|window| {
        let presented = present.as_deref().and_then(PresentHost::presented);
        let faithful = visual.as_deref().map(|visual| &visual.image_handle);
        let frame = sprites
            .iter()
            .find(|(sprite, _)| Some(&sprite.image) == presented || Some(&sprite.image) == faithful)
            .or_else(|| sprites.iter().next());
        let (sprite, transform) = frame?;
        frame_rect(window, sprite, transform)
    });
    let (Some(image), Some(rect)) = (view.image().cloned(), rect) else {
        for (entity, ..) in &nodes {
            commands.entity(entity).despawn();
        }
        for entity in &backdrops {
            commands.entity(entity).despawn();
        }
        return;
    };
    let mut placed = false;
    for (entity, mut node, mut image_node) in &mut nodes {
        if placed {
            commands.entity(entity).despawn();
            continue;
        }
        set_rect(&mut node, rect);
        if image_node.image != image {
            image_node.image = image.clone();
        }
        placed = true;
    }
    if !placed {
        commands.spawn((
            Name::new("enh menu"),
            MenuNode,
            absolute(rect),
            LayoutConfig {
                use_rounding: false,
            },
            ImageNode::new(image),
            GlobalZIndex(MENU_NODE_Z),
        ));
    }
    let mut backdrop = false;
    for entity in &backdrops {
        if backdrop {
            commands.entity(entity).despawn();
        }
        backdrop = true;
    }
    if !backdrop {
        commands.spawn((
            Name::new("enh menu backdrop"),
            MenuBackdrop,
            full_window(),
            BackgroundColor(backdrop_colour()),
            GlobalZIndex(MENU_BACKDROP_Z),
        ));
    }
}

#[cfg(test)]
#[path = "render_tests.rs"]
mod tests;

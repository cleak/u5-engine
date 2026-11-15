//! The Cheats page (Phase B design 7.2.1-7.2.3; build plan M8-02).
//!
//! **Look.** The page is drawn with the game's own pieces, as the menu's
//! frame kit draws its pages (the approved c04f look): IBM.CH glyphs on the
//! 8 px cell grid; a blue band (EGA 1) with a 1 px white rule (EGA 15) on
//! every band pixel that touches a black box, diagonals included; the outer
//! corners carved with `CHROME_CORNER_PROFILE`; one-cell divider bands
//! whose rules turn into the ring's; title and footer tabs closed by the
//! ribbon end caps, as `>Select:<` and `>Calm Winds<` sit on their rules;
//! light grey for headings and help, dark grey for an action that cannot
//! work now, and brown (EGA 6, `#AA5500`) as the one accent, for the word
//! "Cheats" and the "MODIFIED" mark. Everything outside the panel dims the
//! live game (black at 0.6).
//!
//! On the 320x200 frame the page is the whole frame's ring, cells 0-39 by
//! 0-23 (as the menu's wide pages): the top box holds the header, the
//! middle box the quick actions and the locks, the bottom box the result
//! line. A disabled cheat is not drawn and the rows close up (7.2.3).
//!
//! **Keys** (7.2.3). The page owns the keyboard while open (the router's
//! modal input): the bracketed letters queue a quick action, the digits
//! check or uncheck a lock (a checked lock applies once at once), and Esc
//! or Ctrl+F12 closes it. Every action goes through `cheat_apply`; this
//! module never writes the game.
//!
//! **The confirmation** (7.2.1). "Allow cheats?" is a dialog of the same
//! kit; `Y` asks the foundation to turn `cheats.enabled` on at the next
//! idle prompt, `N` or Esc cancels.

use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use u5_runtime::enhancements::{CheatToggles, Toggle};
use u5_runtime::{
    CH_CELL_SIDE, CHROME_CORNER_PROFILE, CHROME_RIBBON_INDEX, CHROME_RULE_INDEX, FixedCellFont,
    PlayState, RibbonCapDirection, ribbon_cap_sprite,
};

use super::apply::{LockKind, QuickAction, lock_off_result};
use super::{CheatAction, CheatSession, MarkerState, Toggles, cheat_toggles, page_available};
use crate::enh::PendingToggleChanges;
use crate::enh::input::OverlayKey;
use crate::enh::overlay::{OverlayId, OverlayRequest, OverlayState};
use crate::{EGA_PALETTE_RGB, VISUAL_PLAY_FRAME_HEIGHT, VISUAL_PLAY_FRAME_WIDTH, VisualState};

// ---------------------------------------------------------------------------
// Engine text (7.2.1, 7.2.3).

/// The title tab.
pub const TITLE: &str = "Cheats";
/// The footer tab.
pub const FOOTER: &str = "Esc: back";
/// The header, before the marker word.
pub const HEADER: &str = "This game:";
pub const HEADER_MODIFIED: &str = "MODIFIED";
pub const HEADER_CLEAN: &str = "unmodified";
pub const HEADER_UNKNOWN: &str = "unknown";
pub const QUICK_HEADING: &str = "Quick actions";
pub const LOCKS_HEADING: &str = "Locks (held once a turn)";
/// The dialog's title tab.
pub const CONFIRM_TITLE: &str = "Allow cheats?";
/// The dialog's text, our own wording (7.2.1), one line a row.
pub const CONFIRM_LINES: [&str; 8] = [
    "Cheats change your party and",
    "the world in ways the",
    "original game never would.",
    "Saves you make afterwards",
    "are marked \"modified\". You",
    "can turn cheats off again",
    "at any time; that does not",
    "unmark a save.",
];
/// The dialog's answers.
pub const CONFIRM_ANSWERS: &str = "[Y] Allow cheats  [N] Cancel";

impl QuickAction {
    /// The key that queues it.
    pub const fn key(self) -> KeyCode {
        match self {
            QuickAction::HealAll => KeyCode::KeyH,
            QuickAction::CurePoison => KeyCode::KeyP,
            QuickAction::ResurrectAll => KeyCode::KeyR,
            QuickAction::MaxGold => KeyCode::KeyG,
            QuickAction::MaxFood => KeyCode::KeyF,
            QuickAction::RefillArrows => KeyCode::KeyA,
            QuickAction::MaxReagents => KeyCode::KeyN,
            QuickAction::RepairHull => KeyCode::KeyS,
        }
    }

    /// The page's item, key and label (engine text).
    pub const fn label(self) -> &'static str {
        match self {
            QuickAction::HealAll => "[H] Heal all",
            QuickAction::CurePoison => "[P] Cure poison",
            QuickAction::ResurrectAll => "[R] Resurrect all",
            QuickAction::MaxGold => "[G] Max gold",
            QuickAction::MaxFood => "[F] Max food",
            QuickAction::RefillArrows => "[A] Refill arrows",
            QuickAction::MaxReagents => "[N] Max reagents",
            QuickAction::RepairHull => "[S] Repair hull",
        }
    }
}

/// The lock a digit key checks.
fn lock_of_key(key: KeyCode) -> Option<LockKind> {
    let index = match key {
        KeyCode::Digit1 | KeyCode::Numpad1 => 0,
        KeyCode::Digit2 | KeyCode::Numpad2 => 1,
        KeyCode::Digit3 | KeyCode::Numpad3 => 2,
        KeyCode::Digit4 | KeyCode::Numpad4 => 3,
        KeyCode::Digit5 | KeyCode::Numpad5 => 4,
        KeyCode::Digit6 | KeyCode::Numpad6 => 5,
        KeyCode::Digit7 | KeyCode::Numpad7 => 6,
        KeyCode::Digit8 | KeyCode::Numpad8 => 7,
        KeyCode::Digit9 | KeyCode::Numpad9 => 8,
        _ => return None,
    };
    Some(LockKind::ALL[index])
}

// ---------------------------------------------------------------------------
// The page's state.

/// Which face the page shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PageMode {
    #[default]
    Main,
    /// The "Allow cheats?" confirmation (7.2.1).
    Confirm,
}

/// The page: its face, its result line and its picture. Exists while some
/// cheat or the watch is on, or while the confirmation is asked.
#[derive(Resource, Debug, Default)]
pub struct CheatPage {
    mode: PageMode,
    /// The bottom strip (7.2.3).
    result: Option<String>,
    /// The picture last drawn, and what it showed.
    image: Option<Handle<Image>>,
    drawn: Option<PageView>,
}

impl CheatPage {
    pub fn mode(&self) -> PageMode {
        self.mode
    }

    pub fn confirming(&self) -> bool {
        self.mode == PageMode::Confirm
    }

    /// The result line showing, if any.
    pub fn result(&self) -> Option<&str> {
        self.result.as_deref()
    }

    pub fn set_result(&mut self, text: String) {
        self.result = Some(text);
    }

    /// The page's picture, once drawn.
    pub fn image(&self) -> Option<&Handle<Image>> {
        self.image.as_ref()
    }

    pub(super) fn open_main(&mut self) {
        self.mode = PageMode::Main;
        self.result = None;
    }

    pub(super) fn open_confirm(&mut self) {
        self.mode = PageMode::Confirm;
        self.result = None;
    }
}

/// The page's keys, while it is the open overlay (7.2.3).
#[allow(clippy::too_many_arguments)]
pub(super) fn page_keys(
    mut commands: Commands,
    mut keys: EventReader<OverlayKey>,
    overlay: Res<OverlayState>,
    keyboard: Option<Res<ButtonInput<KeyCode>>>,
    toggles: Res<Toggles>,
    mut page: ResMut<CheatPage>,
    mut session: Option<ResMut<CheatSession>>,
    mut pending: Option<ResMut<PendingToggleChanges>>,
    mut requests: EventWriter<OverlayRequest>,
) {
    if overlay.open() != Some(OverlayId::Cheats) {
        keys.clear();
        return;
    }
    let ctrl = keyboard.is_some_and(|keyboard| {
        keyboard.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight])
    });
    let cheats = cheat_toggles(toggles.0);
    for OverlayKey(key) in keys.read().copied() {
        let close = key == KeyCode::Escape || (key == KeyCode::F12 && ctrl);
        if page.mode == PageMode::Confirm {
            let answer = match key {
                KeyCode::KeyY => Some(true),
                KeyCode::KeyN => Some(false),
                _ if close => Some(false),
                _ => None,
            };
            let Some(allow) = answer else { continue };
            if allow && let Some(pending) = pending.as_mut() {
                pending.0.push((Toggle::CheatsEnabled, true));
            }
            page.mode = PageMode::Main;
            requests.write(OverlayRequest::Close(OverlayId::Cheats));
            if !page_available(toggles.0) {
                commands.remove_resource::<CheatPage>();
            }
            return;
        }
        if close {
            requests.write(OverlayRequest::Close(OverlayId::Cheats));
            return;
        }
        let Some(session) = session.as_mut() else {
            continue;
        };
        if let Some(quick) = QuickAction::ALL
            .into_iter()
            .find(|quick| quick.key() == key && quick.allowed(&cheats))
        {
            session.queued.push(CheatAction::Quick(quick));
        } else if let Some(lock) = lock_of_key(key).filter(|lock| lock.permitted(&cheats.locks)) {
            let engaged = &mut session.engaged[lock as usize];
            *engaged = !*engaged;
            if *engaged {
                session.queued.push(CheatAction::LockNow(lock));
            } else {
                page.result = Some(lock_off_result(lock));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// What the page shows.

/// One drawn cell run: text from a column in an ink.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Run {
    col: u32,
    row: u32,
    text: String,
    ink: u8,
}

/// Everything the page draws, compared frame to frame so the picture is
/// rebuilt only when it changes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PageView {
    picture: (u32, u32),
    mode: PageMode,
    runs: Vec<Run>,
}

/// The panel's rectangle in cells.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CellRect {
    pub col: u32,
    pub row: u32,
    pub w: u32,
    pub h: u32,
}

impl CellRect {
    const fn new(col: u32, row: u32, w: u32, h: u32) -> Self {
        Self { col, row, w, h }
    }

    const fn right(&self) -> u32 {
        self.col + self.w - 1
    }

    const fn bottom(&self) -> u32 {
        self.row + self.h - 1
    }

    const fn pixels(&self) -> (u32, u32, u32, u32) {
        (
            self.col * CELL,
            self.row * CELL,
            (self.col + self.w) * CELL - 1,
            (self.row + self.h) * CELL - 1,
        )
    }
}

const CELL: u32 = CH_CELL_SIDE as u32;
/// The main page: the whole frame's width, the frame's ring rows (row 24 is
/// black in play).
const PAGE_CELLS: (u32, u32) = (40, 24);
/// The divider rows of the main page, relative to its top: the header box
/// above the first, the result box below the second.
const DIVIDERS: [u32; 2] = [2, 19];
/// The dialog's width: its longest line, a margin each side and the ring.
const DIALOG_W: u32 = 32;
/// The second column of the two-column lists, relative to the panel.
const SECOND_COL: u32 = 21;
/// Ink: white text, light grey headings, dark grey for an action that
/// cannot work now, brown for the accent.
const INK: u8 = CHROME_RULE_INDEX;
const HELP: u8 = 7;
const GREY: u8 = 8;
const ACCENT: u8 = 6;
const PAPER: u8 = 0;
/// The backdrop over the live game: black at 0.6.
const DIM: [u8; 4] = [0, 0, 0, 153];

/// The main page's panel on a `w` by `h` picture: the frame's ring on the
/// 320x200 frame, centred on a wider canvas.
pub(crate) fn page_rect(w: u32, h: u32) -> CellRect {
    let (cols, rows) = (w / CELL, h / CELL);
    let pw = PAGE_CELLS.0.min(cols);
    let ph = PAGE_CELLS.1.min(rows);
    CellRect::new((cols - pw) / 2, (rows.saturating_sub(1) - ph) / 2, pw, ph)
}

/// The dialog's panel, centred on the main page's rectangle.
fn dialog_rect(page: CellRect) -> CellRect {
    let h = CONFIRM_LINES.len() as u32 + 4;
    let w = DIALOG_W.min(page.w);
    CellRect::new(
        page.col + (page.w - w) / 2,
        page.row + (page.h - h) / 2,
        w,
        h,
    )
}

/// Builds the view of the page.
fn page_view(
    picture: (u32, u32),
    mode: PageMode,
    cheats: &CheatToggles,
    session: Option<&CheatSession>,
    state: Option<&PlayState>,
    result: Option<&str>,
) -> PageView {
    let rect = page_rect(picture.0, picture.1);
    let mut runs = Vec::new();
    let mut put = |col: u32, row: u32, text: &str, ink: u8| {
        runs.push(Run {
            col,
            row,
            text: text.to_string(),
            ink,
        });
    };
    if mode == PageMode::Confirm {
        let dialog = dialog_rect(rect);
        for (i, line) in CONFIRM_LINES.iter().enumerate() {
            put(dialog.col + 2, dialog.row + 1 + i as u32, line, INK);
        }
        put(dialog.col + 2, dialog.bottom() - 1, CONFIRM_ANSWERS, INK);
        return PageView {
            picture,
            mode,
            runs,
        };
    }
    let (left, right) = (rect.col + 2, rect.col + SECOND_COL);
    if let Some(session) = session {
        let (word, ink) = match session.header() {
            MarkerState::Modified => (HEADER_MODIFIED, ACCENT),
            MarkerState::Clean => (HEADER_CLEAN, HELP),
            MarkerState::Unknown => (HEADER_UNKNOWN, HELP),
        };
        put(left, rect.row + 1, HEADER, INK);
        put(left + HEADER.len() as u32 + 1, rect.row + 1, word, ink);
    }
    let mut row = rect.row + DIVIDERS[0] + 1;
    let quick: Vec<QuickAction> = QuickAction::ALL
        .into_iter()
        .filter(|quick| quick.allowed(cheats))
        .collect();
    if !quick.is_empty() {
        put(left, row, QUICK_HEADING, HELP);
        row += 1;
        for pair in quick.chunks(2) {
            for (quick, col) in pair.iter().zip([left, right]) {
                let dim = state.is_some_and(|state| quick.impossible_now(state));
                put(col, row, quick.label(), if dim { GREY } else { INK });
            }
            row += 1;
        }
        row += 1;
    }
    let locks: Vec<(usize, LockKind)> = LockKind::ALL
        .into_iter()
        .enumerate()
        .filter(|(_, lock)| lock.permitted(&cheats.locks))
        .collect();
    if !locks.is_empty() {
        put(left, row, LOCKS_HEADING, HELP);
        row += 1;
        for pair in locks.chunks(2) {
            for ((index, lock), col) in pair.iter().zip([left, right]) {
                let checked = session.is_some_and(|session| session.is_engaged(*lock));
                let text = format!(
                    "[{}] {} {}",
                    if checked { 'x' } else { ' ' },
                    index + 1,
                    lock.label()
                );
                put(col, row, &text, INK);
            }
            row += 1;
        }
    }
    if let Some(result) = result {
        let width = (rect.w - 4) as usize;
        let first = rect.row + DIVIDERS[1] + 1;
        for (line_row, line) in (first..rect.bottom()).zip(wrap(result, width)) {
            put(left, line_row, &line, INK);
        }
    }
    PageView {
        picture,
        mode,
        runs,
    }
}

/// Word wrap to `width` cells.
fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split(' ') {
        if !line.is_empty() && line.len() + 1 + word.len() > width {
            lines.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

// ---------------------------------------------------------------------------
// Drawing: the frame kit's pieces.

/// The page's picture, RGBA, transparent until drawn.
pub(crate) struct PageImage {
    pub w: u32,
    pub h: u32,
    pub rgba: Vec<u8>,
}

fn ega(index: u8) -> [u8; 4] {
    let [r, g, b] = EGA_PALETTE_RGB[usize::from(index & 0x0f)];
    [r, g, b, 0xff]
}

impl PageImage {
    fn new(w: u32, h: u32) -> Self {
        Self {
            w,
            h,
            rgba: vec![0; (w * h * 4) as usize],
        }
    }

    fn set(&mut self, x: u32, y: u32, pixel: [u8; 4]) {
        if x >= self.w || y >= self.h {
            return;
        }
        let i = ((y * self.w + x) * 4) as usize;
        self.rgba[i..i + 4].copy_from_slice(&pixel);
    }

    fn fill_cells(&mut self, col: u32, row: u32, w: u32, index: u8) {
        let (x0, y0, x1, y1) = CellRect::new(col, row, w, 1).pixels();
        for y in y0..=y1 {
            for x in x0..=x1 {
                self.set(x, y, ega(index));
            }
        }
    }
}

/// The band, the boxes cut into it on whole cells, the rule on every band
/// pixel touching a box (diagonals included) and the carved corners.
fn draw_panel(img: &mut PageImage, rect: CellRect, dividers: &[u32]) {
    let (x0, y0, x1, y1) = rect.pixels();
    let is_box_cell = |col: u32, row: u32| {
        col > rect.col
            && col < rect.right()
            && row > rect.row
            && row < rect.bottom()
            && !dividers.contains(&(row - rect.row))
    };
    let is_box = |x: i64, y: i64| -> bool {
        if x < i64::from(x0) || y < i64::from(y0) || x > i64::from(x1) || y > i64::from(y1) {
            return false;
        }
        is_box_cell(x as u32 / CELL, y as u32 / CELL)
    };
    for y in y0..=y1 {
        for x in x0..=x1 {
            let (xi, yi) = (i64::from(x), i64::from(y));
            let index = if is_box(xi, yi) {
                PAPER
            } else if (-1..=1).any(|dy| (-1..=1).any(|dx| is_box(xi + dx, yi + dy))) {
                CHROME_RULE_INDEX
            } else {
                CHROME_RIBBON_INDEX
            };
            img.set(x, y, ega(index));
        }
    }
    for (r, first) in CHROME_CORNER_PROFILE.iter().enumerate() {
        let r = r as u32;
        for d in 0..u32::from(*first) {
            for (x, y) in [
                (x0 + d, y0 + r),
                (x1 - d, y0 + r),
                (x0 + d, y1 - r),
                (x1 - d, y1 - r),
            ] {
                img.set(x, y, [0; 4]);
            }
        }
    }
}

/// One 8x8 mask at a cell: set bits in `fg`, clear bits in `bg` when given.
fn draw_mask(img: &mut PageImage, mask: [u8; 8], col: u32, row: u32, fg: u8, bg: Option<u8>) {
    let (bx, by) = (col * CELL, row * CELL);
    for (gy, bits) in mask.iter().enumerate() {
        for gx in 0..CH_CELL_SIDE {
            let index = if bits & (0x80 >> gx) != 0 {
                Some(fg)
            } else {
                bg
            };
            if let Some(index) = index {
                img.set(bx + gx as u32, by + gy as u32, ega(index));
            }
        }
    }
}

fn glyph(font: &FixedCellFont, code: u8) -> [u8; 8] {
    let mut mask = [0u8; 8];
    for (row, bits) in mask.iter_mut().enumerate() {
        *bits = font.glyph_row(code & 0x7f, row).unwrap_or(0);
    }
    mask
}

fn draw_text(img: &mut PageImage, font: &FixedCellFont, col: u32, row: u32, text: &str, ink: u8) {
    for (i, byte) in text.bytes().enumerate() {
        draw_mask(
            img,
            glyph(font, byte),
            col + i as u32,
            row,
            ink,
            Some(PAPER),
        );
    }
}

/// The faithful bracket: the cap's triangle in the band's blue, its two
/// strokes in the rule's white, over black.
fn draw_cap(
    img: &mut PageImage,
    font: &FixedCellFont,
    direction: RibbonCapDirection,
    col: u32,
    row: u32,
) {
    let sprite = ribbon_cap_sprite(font, direction);
    img.fill_cells(col, row, 1, PAPER);
    draw_mask(img, sprite.ribbon, col, row, CHROME_RIBBON_INDEX, None);
    draw_mask(img, sprite.white, col, row, CHROME_RULE_INDEX, None);
}

/// A tab centred on a band row of `rect`: `>text<`.
fn draw_tab(
    img: &mut PageImage,
    font: &FixedCellFont,
    rect: CellRect,
    row: u32,
    text: &str,
    ink: u8,
) {
    let cells = text.len() as u32 + 2;
    let col = rect.col + (rect.w - cells) / 2;
    draw_cap(img, font, RibbonCapDirection::Right, col, row);
    draw_text(img, font, col + 1, row, text, ink);
    draw_cap(img, font, RibbonCapDirection::Left, col + cells - 1, row);
}

/// Every pixel nothing drew dims the game behind.
fn dim_backdrop(img: &mut PageImage) {
    for pixel in img.rgba.as_chunks_mut::<4>().0 {
        if pixel[3] == 0 {
            pixel.copy_from_slice(&DIM);
        }
    }
}

/// Draws `view` with `font`.
pub(crate) fn draw_page(view: &PageView, font: &FixedCellFont) -> PageImage {
    let (w, h) = view.picture;
    let mut img = PageImage::new(w, h);
    let rect = page_rect(w, h);
    match view.mode {
        PageMode::Main => {
            draw_panel(&mut img, rect, &DIVIDERS);
            draw_tab(&mut img, font, rect, rect.row, TITLE, ACCENT);
            draw_tab(&mut img, font, rect, rect.bottom(), FOOTER, INK);
        }
        PageMode::Confirm => {
            let dialog = dialog_rect(rect);
            draw_panel(&mut img, dialog, &[]);
            draw_tab(&mut img, font, dialog, dialog.row, CONFIRM_TITLE, INK);
        }
    }
    for run in &view.runs {
        draw_text(&mut img, font, run.col, run.row, &run.text, run.ink);
    }
    dim_backdrop(&mut img);
    img
}

// ---------------------------------------------------------------------------
// Showing it.

/// The page's node over the frame sprite.
#[derive(Component, Debug)]
pub struct CheatPageNode;

/// Above the frame and the gem map's nodes, below the toast.
const PAGE_Z: i32 = i32::MAX - 4;

/// Run condition: the page exists, or its node is left to remove.
pub(super) fn page_present_due(
    page: Option<Res<CheatPage>>,
    nodes: Query<(), With<CheatPageNode>>,
) -> bool {
    page.is_some() || !nodes.is_empty()
}

/// The frame sprite's rectangle in window coordinates and the size of the
/// picture it shows.
fn frame_sprite(
    windows: &Query<&Window>,
    sprites: &Query<(&Sprite, &Transform)>,
    images: &Assets<Image>,
) -> (Option<Rect>, Option<(u32, u32)>) {
    let Some((sprite, transform)) = sprites
        .iter()
        .find(|(sprite, _)| sprite.custom_size.is_some())
    else {
        return (None, None);
    };
    let picture = images.get(&sprite.image).map(|image| {
        let size = image.size();
        (size.x, size.y)
    });
    let rect = windows.single().ok().and_then(|window| {
        let size = sprite.custom_size? * transform.scale.truncate();
        let centre = Vec2::new(
            window.resolution.width() / 2.0 + transform.translation.x,
            window.resolution.height() / 2.0 - transform.translation.y,
        );
        Some(Rect::from_center_size(centre, size))
    });
    (rect, picture)
}

fn rgba_image(img: PageImage) -> Image {
    let mut image = Image::new(
        Extent3d {
            width: img.w,
            height: img.h,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        img.rgba,
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

/// Draws the page while it is the open overlay and removes it otherwise,
/// in `EnhSet::Present`. The picture is rebuilt only when what it shows
/// changed.
#[allow(clippy::too_many_arguments)]
pub(super) fn page_present(
    mut commands: Commands,
    page: Option<ResMut<CheatPage>>,
    overlay: Option<Res<OverlayState>>,
    toggles: Option<Res<Toggles>>,
    session: Option<Res<CheatSession>>,
    visual: Option<Res<VisualState>>,
    windows: Query<&Window>,
    sprites: Query<(&Sprite, &Transform)>,
    mut images: ResMut<Assets<Image>>,
    mut nodes: Query<(Entity, &mut Node, &mut ImageNode), With<CheatPageNode>>,
) {
    let open = overlay.is_some_and(|overlay| overlay.open() == Some(OverlayId::Cheats));
    let (Some(mut page), true, Some(toggles), Some(visual)) = (page, open, toggles, visual) else {
        for (entity, ..) in &nodes {
            commands.entity(entity).despawn();
        }
        return;
    };
    let (rect, picture) = frame_sprite(&windows, &sprites, &images);
    let picture = picture.unwrap_or((VISUAL_PLAY_FRAME_WIDTH, VISUAL_PLAY_FRAME_HEIGHT));
    let view = page_view(
        picture,
        page.mode,
        &cheat_toggles(toggles.0),
        session.as_deref(),
        Some(&visual.state),
        page.result.as_deref(),
    );
    if page.drawn.as_ref() != Some(&view) || page.image.is_none() {
        let image = rgba_image(draw_page(&view, &visual.text_font));
        match page.image.clone() {
            Some(handle) if images.contains(&handle) => {
                images.insert(&handle, image);
            }
            _ => page.image = Some(images.add(image)),
        }
        page.drawn = Some(view);
    }
    let Some(handle) = page.image.clone() else {
        return;
    };
    let mut placed = false;
    for (entity, mut node, mut image_node) in &mut nodes {
        if placed {
            commands.entity(entity).despawn();
            continue;
        }
        if let Some(rect) = rect {
            place(&mut node, rect);
        }
        if image_node.image != handle {
            image_node.image = handle.clone();
        }
        placed = true;
    }
    if !placed {
        let mut node = Node::default();
        if let Some(rect) = rect {
            place(&mut node, rect);
        }
        commands.spawn((
            Name::new("enh cheats page"),
            CheatPageNode,
            node,
            ImageNode::new(handle),
            GlobalZIndex(PAGE_Z),
        ));
    }
}

/// The main page's view for `cheats` and `session`, at the exploration
/// prompt with no result line (tests and review shots).
#[cfg(test)]
pub(crate) fn page_view_for_tests(
    picture: (u32, u32),
    cheats: &CheatToggles,
    session: &CheatSession,
) -> PageView {
    page_view(picture, PageMode::Main, cheats, Some(session), None, None)
}

#[cfg(test)]
impl PageView {
    /// Every text run, in drawing order.
    pub(crate) fn texts(&self) -> Vec<String> {
        self.runs.iter().map(|run| run.text.clone()).collect()
    }
}

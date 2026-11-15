//! The quest journal page in the Bevy shell (Phase B design 5.2.2-5.2.9,
//! 5.3.4; build plan M5-03 and M5-04).
//!
//! **Zero footprint.** [`QuestJournalPlugin`] is added by
//! [`add_quest_journal`] only when `journal.page` or `journal.side_panel`
//! is on at launch, or the menu overlay is (it can switch the page on in
//! play). That call sits inside the foundation's plugin, which is absent
//! under Classic and under the parity lock. [`QuestJournalUi`] exists only
//! while `journal.page` is on, and every page system runs only while it
//! exists. The page names no faithful system and adds no `PlayState` field.
//!
//! **Input.** The page never reads the keyboard. Ctrl+J is the foundation
//! router's `[keys] journal` binding: the router honours it under the order
//! rule and sends [`EnhActionEvent`]`(OpenJournal)`, and the page asks the
//! overlay model to open [`OverlayId::Journal`], which opens only at the
//! idle exploration prompt and toasts otherwise. While the page is open the
//! router hands every key to it as an [`OverlayKey`]; Esc or Ctrl+J closes
//! it. The page keeps no open flag: it is open exactly while
//! [`OverlayState`] says so.
//!
//! **Drawing.** The page is its own 320x200 image on its own sprite at
//! [`PAGE_Z`], hidden while closed, so the play image is never written.
//! [`align_journal_page`] puts it over the frame sprite every frame, after
//! the faithful fit and the side-space layout: the same rectangle in the
//! faithful 4:3 frame and beside a granted panel, and the game's own pixel
//! scale, centred, on a grow-left canvas. [`paint_quest_journal_page`]
//! re-derives the model while the page is open and repaints only when it
//! changed, with the game's IBM.CH font and frame pieces ([`paint_page`]).
//!
//! **Content.** [`page_cells`] lays the page out on the 40x25 grid, inside
//! columns 2-37: the summary header, the section switcher (only sections
//! with a row cycle), the column headings, [`VISIBLE_ROWS`] rows in the
//! section's [`columns`] with the selected row on a blue bar, and the
//! selected row's details on rows 22-23. Tier 4 answers are masked as
//! `[hidden]` (dark grey on blue) until Enter or Space; the reveals live in
//! [`QuestJournalUi`] only, are cleared whenever the page closes and are
//! never saved.
//!
//! **The side panel** (M5-05, [`panel`]): a [`JournalPanelProvider`] in the
//! foundation's panel registry while `journal.side_panel` is on, drawn by
//! the foundation's cell-panel host; empty while the page is open, never
//! above tier 2 and never a mantra.
//!
//! **Tabs (the M6 seam).** Chapter 8 turns the page into several tabs.
//! [`QuestJournalUi`] keeps a tab list that holds only [`JournalTab::Quests`]
//! here; no tab strip is drawn while there is one tab.

use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use u5_runtime::enhancements::{JournalSettings, MantraDisplay, SpoilerTier, Toggle};
use u5_runtime::gameplay_chrome::{
    CHROME_CORNER_PROFILE, ChromePalette, RibbonCapDirection, paint_ribbon_cap,
};
use u5_runtime::quest_journal::{
    JournalModel, JournalRow, JournalSection, QuestSnapshot, RowKey, SectionId, ShrinePhase,
    StatusMark, derive, text,
};
use u5_runtime::{CH_CELL_SIDE, EGA_PALETTE_RGB, FixedCellFont, STOCK_EGA_BROWN};

use crate::enh::input::{EnhAction, EnhActionEvent, OverlayKey, menu_overlay_on};
use crate::enh::overlay::{OverlayId, OverlayRequest, OverlayState, apply_overlay_requests};
use crate::enh::side::{FrameImageSize, PanelRegistry, draw_side_panels};
use crate::enh::{EnhLaunch, EnhSet, Toggles, VisualLaunchConfig};
use crate::{VISUAL_PLAY_FRAME_HEIGHT, VISUAL_PLAY_FRAME_WIDTH, VisualState};

/// The page sprite's depth: above the frame sprite (z = 0).
pub const PAGE_Z: f32 = 10.0;

/// The page image, the 40x25 cell grid of the play frame.
pub const PAGE_WIDTH: usize = VISUAL_PLAY_FRAME_WIDTH as usize;
pub const PAGE_HEIGHT: usize = VISUAL_PLAY_FRAME_HEIGHT as usize;

/// Row 0: the top band, with the title tab.
pub const TITLE_ROW: u8 = 0;
/// Row 1: the summary header.
pub const HEADER_ROW: u8 = 1;
/// The band between the header and the body.
pub const UPPER_DIVIDER_ROW: u8 = 2;
/// Rows 3-20: the section body: the section switcher, the column
/// headings and [`VISIBLE_ROWS`] rows.
pub const BODY_FIRST_ROW: u8 = 3;
pub const SECTION_ROW: u8 = BODY_FIRST_ROW;
pub const HEADING_ROW: u8 = 4;
pub const ROWS_FIRST_ROW: u8 = 5;
pub const BODY_LAST_ROW: u8 = 20;
/// The rows a section shows at once; PgUp/PgDn move by this many.
pub const VISIBLE_ROWS: usize = (BODY_LAST_ROW - ROWS_FIRST_ROW + 1) as usize;
/// The band between the body and the details.
pub const LOWER_DIVIDER_ROW: u8 = 21;
/// Rows 22-23: the selected row's details.
pub const DETAIL_FIRST_ROW: u8 = 22;
pub const DETAIL_LAST_ROW: u8 = 23;
/// Row 24: the bottom band, with the key help tab.
pub const KEY_HELP_ROW: u8 = 24;

/// Text keeps one empty cell from each side rule: columns 2-37, 36 cells.
pub const TEXT_COLUMN: u8 = 2;
pub const LAST_TEXT_COLUMN: u8 = 37;
pub const TEXT_CELLS: usize = (LAST_TEXT_COLUMN - TEXT_COLUMN + 1) as usize;

/// The mantra cell's fixed width: `[hidden]`, longer than any mantra, so a
/// reveal never moves the row (5.2.7).
pub const MANTRA_CELLS: usize = 8;

/// IBM.CH's arrow glyphs: the section switcher and the scroll marks.
const GLYPH_UP: char = '\u{18}';
const GLYPH_DOWN: char = '\u{19}';
const GLYPH_RIGHT: char = '\u{1a}';
const GLYPH_LEFT: char = '\u{1b}';

/// The frame's rules: the left and right rule columns and the top and
/// bottom rule rows, as the gameplay frame draws its own at x = 7 and
/// x = 312.
const RULE_LEFT_X: usize = 7;
const RULE_RIGHT_X: usize = PAGE_WIDTH - 8;
const RULE_TOP_Y: usize = 7;
const RULE_BOTTOM_Y: usize = KEY_HELP_ROW as usize * CH_CELL_SIDE;

/// An RGB colour.
pub type Rgb = [u8; 3];

/// An EGA palette colour.
pub const fn ega(index: u8) -> Rgb {
    EGA_PALETTE_RGB[(index & 0x0f) as usize]
}

/// The page's colours: the gameplay frame's blue band and white rule
/// (`ChromePalette::EGA`), black, light grey body text (5.2.5) and brown
/// `#AA5500`, the one accent. Brown is the stock EGA brown, not palette
/// slot 6, which this game shows as dark yellow.
const BAND: Rgb = ega(ChromePalette::EGA.chrome);
const WHITE: Rgb = ega(ChromePalette::EGA.accent);
const BLACK: Rgb = ega(ChromePalette::EGA.background);
const BODY: Rgb = ega(7);
const DARK_GREY: Rgb = ega(8);
/// The `[hidden]` cell's paper and the selected row's bar: EGA 1, the
/// frame's own blue.
const MASK_PAPER: Rgb = ega(1);
pub const BROWN: Rgb = STOCK_EGA_BROWN;

/// The key help (5.2.4), with IBM.CH's own arrows so the tab stays short
/// and the bottom rule shows either side of it: Esc while nothing on the
/// row can be revealed, Enter while a masked answer can.
pub const KEY_HELP: &str = "\u{18}\u{19} row  \u{1b}\u{1a} section  Esc close";
pub const KEY_HELP_REVEAL: &str = "\u{18}\u{19} row  \u{1b}\u{1a} section  Enter reveal";

/// One tab of the journal page. Chapter 8 adds Clues, Topics and Notes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum JournalTab {
    Quests,
}

/// The page's state. Present exactly while `journal.page` is on.
#[derive(Resource, Debug)]
pub struct QuestJournalUi {
    settings: JournalSettings,
    tabs: Vec<JournalTab>,
    tab: usize,
    /// The section shown. Kept by id, so a section that appears or goes
    /// while the page is open does not move the player to another one.
    section: SectionId,
    /// The selected row and the first row shown, in the section's rows.
    cursor: usize,
    scroll: usize,
    /// The answers revealed since the page opened. Cleared on close and
    /// never saved (5.2.7).
    revealed: Vec<RowKey>,
    page_image: Option<Handle<Image>>,
    page_entity: Option<Entity>,
    last_model: Option<JournalModel>,
    /// Whether the page was showing on the last frame.
    shown: bool,
    dirty: bool,
}

impl QuestJournalUi {
    pub fn new(settings: JournalSettings) -> Self {
        Self {
            settings,
            tabs: vec![JournalTab::Quests],
            tab: 0,
            section: SectionId::Virtues,
            cursor: 0,
            scroll: 0,
            revealed: Vec::new(),
            page_image: None,
            page_entity: None,
            last_model: None,
            shown: false,
            dirty: true,
        }
    }

    pub fn settings(&self) -> JournalSettings {
        self.settings
    }

    /// The page's tabs, in strip order.
    pub fn tabs(&self) -> &[JournalTab] {
        &self.tabs
    }

    /// The tab showing.
    pub fn tab(&self) -> JournalTab {
        self.tabs[self.tab]
    }

    /// Whether a tab strip is drawn: only with more than one tab.
    pub fn draws_tab_strip(&self) -> bool {
        self.tabs.len() > 1
    }

    /// The page's own image, once its sprite exists.
    pub fn page_image(&self) -> Option<&Handle<Image>> {
        self.page_image.as_ref()
    }

    pub fn page_entity(&self) -> Option<Entity> {
        self.page_entity
    }

    /// The model the page shows (`None` until it is first opened).
    pub fn model(&self) -> Option<&JournalModel> {
        self.last_model.as_ref()
    }

    /// The section shown.
    pub fn section_id(&self) -> SectionId {
        self.section
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn scroll(&self) -> usize {
        self.scroll
    }

    /// The rows whose answer is revealed (empty whenever the page is
    /// closed).
    pub fn revealed(&self) -> &[RowKey] {
        &self.revealed
    }

    /// The section shown, from the last model.
    pub fn current_section(&self) -> Option<&JournalSection> {
        self.last_model.as_ref()?.section(self.section)
    }

    fn row_count(&self) -> usize {
        self.current_section()
            .map_or(0, |section| section.rows.len())
    }

    /// The sections in the Left/Right cycle: the model's, which holds only
    /// sections with a row at its tier (5.2.8).
    pub fn cycle(&self) -> Vec<SectionId> {
        self.last_model.as_ref().map_or_else(Vec::new, |model| {
            model.sections.iter().map(|section| section.id).collect()
        })
    }

    /// Takes a newly derived model: the section stays if it still has rows
    /// (else the first section), and the cursor stays on the same row.
    fn take_model(&mut self, model: JournalModel) {
        let row_key = self
            .current_section()
            .and_then(|section| section.rows.get(self.cursor))
            .map(|row| row.key);
        if model.section(self.section).is_none() {
            self.section = model
                .sections
                .first()
                .map_or(SectionId::Virtues, |section| section.id);
            self.cursor = 0;
            self.scroll = 0;
        } else if let Some(at) = row_key.and_then(|key| {
            model
                .section(self.section)
                .and_then(|section| section.rows.iter().position(|row| row.key == key))
        }) {
            self.cursor = at;
        }
        self.last_model = Some(model);
        self.clamp();
        self.dirty = true;
    }

    /// Keeps the cursor on a row and in view.
    fn clamp(&mut self) {
        let rows = self.row_count();
        self.cursor = self.cursor.min(rows.saturating_sub(1));
        self.scroll = self.scroll.min(rows.saturating_sub(VISIBLE_ROWS));
        if self.cursor < self.scroll {
            self.scroll = self.cursor;
        } else if self.cursor >= self.scroll + VISIBLE_ROWS {
            self.scroll = self.cursor + 1 - VISIBLE_ROWS;
        }
    }

    /// Up/Down (a negative step goes up); no wrap.
    fn move_cursor(&mut self, step: isize) {
        self.cursor = self.cursor.saturating_add_signed(step);
        self.clamp();
    }

    /// PgUp/PgDn: the view and the cursor move a page.
    fn page(&mut self, pages: isize) {
        let step = pages * VISIBLE_ROWS as isize;
        let rows = self.row_count();
        self.scroll = self
            .scroll
            .saturating_add_signed(step)
            .min(rows.saturating_sub(VISIBLE_ROWS));
        self.cursor = self.cursor.saturating_add_signed(step);
        self.clamp();
    }

    fn home(&mut self) {
        self.cursor = 0;
        self.clamp();
    }

    fn end(&mut self) {
        self.cursor = usize::MAX;
        self.clamp();
    }

    /// Left/Right, Tab/Shift+Tab: the previous or next section in the
    /// cycle, wrapping.
    fn step_section(&mut self, step: isize) {
        let cycle = self.cycle();
        let Some(at) = cycle.iter().position(|id| *id == self.section) else {
            return;
        };
        let next = (at as isize + step).rem_euclid(cycle.len() as isize) as usize;
        self.show_section(cycle[next]);
    }

    /// A digit: the section with that place in the fixed order, if it is in
    /// the cycle; otherwise nothing.
    fn jump(&mut self, digit: usize) {
        if let Some(id) = digit
            .checked_sub(1)
            .and_then(|place| SectionId::ALL.get(place))
            && self.cycle().contains(id)
        {
            self.show_section(*id);
        }
    }

    fn show_section(&mut self, id: SectionId) {
        if id != self.section {
            self.section = id;
            self.cursor = 0;
            self.scroll = 0;
        }
    }

    /// Enter/Space: reveals the selected row's masked answer.
    fn reveal(&mut self) {
        let mantras = self.settings.mantras;
        let Some(section) = self.current_section() else {
            return;
        };
        let Some(row) = section.rows.get(self.cursor) else {
            return;
        };
        if masked_answer(section.id, row, mantras) && !self.revealed.contains(&row.key) {
            let key = row.key;
            self.revealed.push(key);
        }
    }

    /// The view one paint draws.
    pub fn view(&self) -> PageView<'_> {
        PageView {
            model: self.last_model.as_ref(),
            section: self.current_section(),
            cursor: self.cursor,
            scroll: self.scroll,
            revealed: &self.revealed,
            mantras: self.settings.mantras,
            tab_strip: self.tabs.len() > 1,
        }
    }
}

/// Whether a row has a tier 4 answer the page masks: a mantra (unless
/// `mantras = "never"` blanks the column), a Word of Power or a flame.
pub fn masked_answer(section: SectionId, row: &JournalRow, mantras: MantraDisplay) -> bool {
    row.secret.is_some() && !(section == SectionId::Virtues && mantras == MantraDisplay::Never)
}

/// Marks the page sprite.
#[derive(Component, Debug)]
pub struct JournalPageSprite;

/// Whether the journal plugin belongs in an app with these settings.
pub fn journal_wanted(launch: &VisualLaunchConfig) -> bool {
    launch.enhancements.journal().any_on() || menu_overlay_on(launch.enhancements.toggles)
}

/// Adds [`QuestJournalPlugin`] when [`journal_wanted`]. Called from the
/// foundation's plugin after the router and the overlay model.
pub(crate) fn add_quest_journal(app: &mut App, launch: &VisualLaunchConfig) {
    if journal_wanted(launch) {
        app.add_plugins(QuestJournalPlugin {
            settings: launch.enhancements.journal(),
        });
    }
}

/// The quest journal's plugin (5.3.4).
pub struct QuestJournalPlugin {
    pub settings: JournalSettings,
}

impl Plugin for QuestJournalPlugin {
    fn build(&self, app: &mut App) {
        if self.settings.page {
            app.insert_resource(QuestJournalUi::new(self.settings));
        }
        // The side panel (5.3.5): a provider in the foundation's registry
        // only while `journal.side_panel` is on.
        if self.settings.side_panel {
            panel::register_journal_panel(app.world_mut(), self.settings.side);
        }
        app.add_systems(
            Update,
            panel::copy_journal_page_open
                .in_set(EnhSet::Present)
                .after(EnhSet::Observe)
                .before(draw_side_panels)
                .run_if(resource_exists::<JournalPanelPageOpen>),
        );
        if app.world().contains_resource::<PanelRegistry>() {
            app.add_systems(
                Update,
                panel::follow_journal_panel_toggle
                    .run_if(resource_exists_and_changed::<Toggles>)
                    .in_set(EnhSet::Apply),
            );
        }
        // The page opens and closes through the overlay model, which is in
        // the app whenever the page (or the menu overlay) is on.
        if app.world().contains_resource::<OverlayState>() {
            app.add_systems(
                Update,
                journal_page_input
                    .before(apply_overlay_requests)
                    .in_set(EnhSet::Apply)
                    .run_if(resource_exists::<QuestJournalUi>),
            );
        }
        app.add_systems(
            Update,
            follow_journal_toggle
                .run_if(resource_exists_and_changed::<Toggles>)
                .in_set(EnhSet::Apply),
        )
        .add_systems(
            Update,
            (paint_quest_journal_page, align_journal_page)
                .chain()
                .in_set(EnhSet::Present)
                // The real chains already order `Present` after `Observe`
                // (through the frame fit); a close there hides the page on
                // the same frame.
                .after(EnhSet::Observe)
                .distributive_run_if(resource_exists::<QuestJournalUi>),
        );
    }
}

/// Inserts or removes [`QuestJournalUi`] when `journal.page` changes in
/// play (from the menu overlay), with the launch's other settings.
fn follow_journal_toggle(
    mut commands: Commands,
    toggles: Res<Toggles>,
    launch: Option<Res<EnhLaunch>>,
    ui: Option<Res<QuestJournalUi>>,
    mut images: ResMut<Assets<Image>>,
) {
    let on = toggles.0.on(Toggle::JournalPage);
    match (on, ui) {
        (true, None) => {
            let mut settings = launch
                .map(|launch| launch.0.enhancements.journal())
                .unwrap_or_default();
            settings.page = true;
            commands.insert_resource(QuestJournalUi::new(settings));
        }
        (false, Some(ui)) => {
            if let Some(entity) = ui.page_entity {
                commands.entity(entity).despawn();
            }
            if let Some(image) = &ui.page_image {
                images.remove(image);
            }
            commands.remove_resource::<QuestJournalUi>();
        }
        _ => {}
    }
}

/// Opens the page on the router's Ctrl+J, closes it on Esc or Ctrl+J while
/// it is open (5.2.2). Only the overlay model opens it, at the idle
/// exploration prompt; elsewhere it refuses with a toast.
pub(crate) fn journal_page_input(
    mut actions: EventReader<EnhActionEvent>,
    mut keys: EventReader<OverlayKey>,
    keyboard: Option<Res<ButtonInput<KeyCode>>>,
    state: Res<OverlayState>,
    mut ui: ResMut<QuestJournalUi>,
    mut requests: EventWriter<OverlayRequest>,
) {
    let open = state.open() == Some(OverlayId::Journal);
    for EnhActionEvent(action) in actions.read() {
        if *action == EnhAction::OpenJournal && !open {
            requests.write(OverlayRequest::Open(OverlayId::Journal));
        }
    }
    let ctrl = keyboard
        .as_ref()
        .is_some_and(|keys| keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]));
    let shift = keyboard
        .as_ref()
        .is_some_and(|keys| keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]));
    let mut close = false;
    for OverlayKey(key) in keys.read() {
        if !open || close {
            continue;
        }
        match key {
            KeyCode::Escape => close = true,
            KeyCode::KeyJ if ctrl => close = true,
            KeyCode::ArrowUp => ui.move_cursor(-1),
            KeyCode::ArrowDown => ui.move_cursor(1),
            KeyCode::PageUp => ui.page(-1),
            KeyCode::PageDown => ui.page(1),
            KeyCode::Home => ui.home(),
            KeyCode::End => ui.end(),
            KeyCode::ArrowLeft => ui.step_section(-1),
            KeyCode::ArrowRight => ui.step_section(1),
            KeyCode::Tab => ui.step_section(if shift { -1 } else { 1 }),
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => ui.reveal(),
            key => match section_digit(*key) {
                Some(digit) => ui.jump(digit),
                None => continue,
            },
        }
        ui.dirty = true;
    }
    if close {
        requests.write(OverlayRequest::Close(OverlayId::Journal));
        ui.revealed.clear();
        ui.dirty = true;
    }
}

/// 1-7 on the main row or the keypad.
fn section_digit(key: KeyCode) -> Option<usize> {
    Some(match key {
        KeyCode::Digit1 | KeyCode::Numpad1 => 1,
        KeyCode::Digit2 | KeyCode::Numpad2 => 2,
        KeyCode::Digit3 | KeyCode::Numpad3 => 3,
        KeyCode::Digit4 | KeyCode::Numpad4 => 4,
        KeyCode::Digit5 | KeyCode::Numpad5 => 5,
        KeyCode::Digit6 | KeyCode::Numpad6 => 6,
        KeyCode::Digit7 | KeyCode::Numpad7 => 7,
        _ => return None,
    })
}

/// A black 320x200 image with the play image's nearest sampler.
fn page_image() -> Image {
    let mut image = Image::new(
        Extent3d {
            width: VISUAL_PLAY_FRAME_WIDTH,
            height: VISUAL_PLAY_FRAME_HEIGHT,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        vec![0; PAGE_WIDTH * PAGE_HEIGHT * 4],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::nearest();
    image
}

/// Spawns the page sprite on first use, shows it while the journal is the
/// open overlay, re-derives the model while it is open and repaints the
/// image only when something changed.
pub(crate) fn paint_quest_journal_page(
    mut commands: Commands,
    mut ui: ResMut<QuestJournalUi>,
    overlay: Option<Res<OverlayState>>,
    visual: Option<Res<VisualState>>,
    mut images: ResMut<Assets<Image>>,
    mut sprites: Query<&mut Visibility, With<JournalPageSprite>>,
) {
    let ui = &mut *ui;
    if ui.page_entity.is_none() {
        let handle = images.add(page_image());
        let entity = commands
            .spawn((
                Sprite {
                    image: handle.clone(),
                    custom_size: Some(Vec2::new(
                        VISUAL_PLAY_FRAME_WIDTH as f32,
                        VISUAL_PLAY_FRAME_HEIGHT as f32,
                    )),
                    ..default()
                },
                Transform::from_xyz(0.0, 0.0, PAGE_Z),
                Visibility::Hidden,
                JournalPageSprite,
            ))
            .id();
        ui.page_image = Some(handle);
        ui.page_entity = Some(entity);
    }
    let open = overlay.is_some_and(|overlay| overlay.open() == Some(OverlayId::Journal));
    if let Some(mut visibility) = ui
        .page_entity
        .and_then(|entity| sprites.get_mut(entity).ok())
    {
        let wanted = if open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if *visibility != wanted {
            *visibility = wanted;
        }
    }
    if !open {
        // Closing masks every answer again (5.2.7).
        if ui.shown {
            ui.revealed.clear();
        }
        ui.shown = false;
        return;
    }
    if !ui.shown {
        ui.shown = true;
        ui.dirty = true;
    }
    let Some(visual) = visual else {
        return;
    };
    let model = derive(
        &QuestSnapshot::from_play_state(&visual.state),
        ui.settings.tier,
    );
    if ui.last_model.as_ref() != Some(&model) {
        ui.take_model(model);
    }
    if !ui.dirty {
        return;
    }
    let Some(image) = ui
        .page_image
        .as_ref()
        .and_then(|handle| images.get_mut(handle))
    else {
        return;
    };
    let Some(rgba) = image.data.as_mut() else {
        return;
    };
    paint_page(rgba, &visual.text_font, &ui.view());
    ui.dirty = false;
}

/// The page's rectangle over a frame sprite of `frame_size` (centred on
/// `centre`) that shows an image of `image` pixels: the frame's own
/// rectangle when that image is the 320x200 frame, otherwise 320x200 of
/// its pixels, centred (a grow-left canvas).
pub fn page_rect(frame_size: Vec2, centre: Vec2, image: UVec2) -> (Vec2, Vec2) {
    let image = image.max(UVec2::ONE).as_vec2();
    let share = Vec2::new(
        VISUAL_PLAY_FRAME_WIDTH as f32,
        VISUAL_PLAY_FRAME_HEIGHT as f32,
    ) / image;
    (frame_size * share, centre)
}

/// Puts the page sprite over the frame sprite, after the faithful fit and
/// the side-space layout have placed it this frame.
pub(crate) fn align_journal_page(
    ui: Res<QuestJournalUi>,
    image: Option<Res<FrameImageSize>>,
    frames: Query<(&Sprite, &Transform), Without<JournalPageSprite>>,
    mut pages: Query<(&mut Sprite, &mut Transform), With<JournalPageSprite>>,
) {
    let Some(entity) = ui.page_entity else {
        return;
    };
    let Ok((mut sprite, mut transform)) = pages.get_mut(entity) else {
        return;
    };
    let Some((frame, frame_transform)) = frames.iter().next() else {
        return;
    };
    let Some(frame_size) = frame.custom_size else {
        return;
    };
    let (size, centre) = page_rect(
        frame_size,
        frame_transform.translation.truncate(),
        FrameImageSize::image(image.as_deref()),
    );
    if sprite.custom_size != Some(size) {
        sprite.custom_size = Some(size);
    }
    if transform.translation.truncate() != centre {
        transform.translation.x = centre.x;
        transform.translation.y = centre.y;
    }
}

/// What one paint of the page draws.
#[derive(Clone, Copy, Debug)]
pub struct PageView<'a> {
    /// `None`: the empty page, the frame and its tabs only.
    pub model: Option<&'a JournalModel>,
    pub section: Option<&'a JournalSection>,
    /// The selected row and the first row shown.
    pub cursor: usize,
    pub scroll: usize,
    /// The rows whose answer is revealed.
    pub revealed: &'a [RowKey],
    pub mantras: MantraDisplay,
    /// Chapter 8's strip, drawn only with more than one tab. M5 never sets
    /// it.
    pub tab_strip: bool,
}

impl<'a> PageView<'a> {
    /// `section` of `model`, its first row selected, nothing revealed.
    pub fn new(model: Option<&'a JournalModel>, section: Option<&'a JournalSection>) -> Self {
        Self {
            model,
            section,
            cursor: 0,
            scroll: 0,
            revealed: &[],
            mantras: MantraDisplay::Reveal,
            tab_strip: false,
        }
    }

    fn selected(&self) -> Option<&'a JournalRow> {
        self.section?.rows.get(self.cursor)
    }

    fn tier(&self) -> SpoilerTier {
        self.model.map_or(SpoilerTier::Goals, |model| model.tier)
    }
}

/// What a [`Cell`] is drawn over.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Paper {
    /// The selected row's blue bar: the text columns and half a cell more
    /// each side, so the bar keeps clear of the side rules.
    Bar,
    /// A masked answer's blue, one pixel short of the cell's foot so masks
    /// on rows one above the other stay apart.
    Mask,
}

/// One run of text on the cell grid, over an optional paper.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cell {
    pub column: u8,
    pub row: u8,
    pub text: String,
    pub ink: Rgb,
    /// Filled under `text` first; `None` keeps what is there.
    pub paper: Option<Paper>,
}

impl Cell {
    fn new(column: u8, row: u8, text: impl Into<String>, ink: Rgb) -> Self {
        Self {
            column,
            row,
            text: text.into(),
            ink,
            paper: None,
        }
    }

    fn on(mut self, paper: Paper) -> Self {
        self.paper = Some(paper);
        self
    }

    /// The cells it covers.
    pub fn width(&self) -> usize {
        self.text.chars().count()
    }

    /// Its last column.
    pub fn last_column(&self) -> usize {
        (self.column as usize + self.width()).saturating_sub(1)
    }
}

/// Where one section's columns start. A column of `None` is not drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Columns {
    pub label: u8,
    /// The marker; the restored `r` or the ruined `!` goes one cell right.
    pub mark: Option<u8>,
    pub value: Option<u8>,
    /// The Shadowlords' Shard column.
    pub note: Option<u8>,
    /// Tier 4's fixed 8-cell mantra column (Virtues only).
    pub mantra: Option<u8>,
}

/// A section's columns (5.2.4-5.2.8) inside the 36 text cells. Virtue
/// names are at most 12 cells, dungeon names 8, regalia and tools 14, party
/// names 8 and towns 24.
pub fn columns(section: SectionId, tier: SpoilerTier, row: Option<&JournalRow>) -> Columns {
    let at = |label, mark: Option<u8>, value: Option<u8>| Columns {
        label,
        mark,
        value,
        note: None,
        mantra: None,
    };
    match section {
        SectionId::Virtues if tier == SpoilerTier::Full => Columns {
            mantra: Some(15),
            ..at(2, Some(24), Some(27))
        },
        SectionId::Virtues => at(2, Some(15), Some(18)),
        SectionId::Dungeons => at(2, Some(15), Some(17)),
        SectionId::Shadowlords => Columns {
            note: Some(26),
            ..at(2, Some(12), Some(14))
        },
        SectionId::Regalia | SectionId::Items => at(2, Some(17), Some(19)),
        SectionId::People => match row.map(|row| row.key) {
            Some(RowKey::Companion(_)) | None => at(2, Some(11), Some(13)),
            // The counts and karma: a town name may take 24 cells.
            Some(_) => at(2, None, Some(27)),
        },
        // The checklist: the marker is the box.
        SectionId::Endgame => at(4, Some(2), None),
    }
}

/// The column headings of a section, on [`HEADING_ROW`].
fn headings(section: SectionId, tier: SpoilerTier) -> Vec<(u8, &'static str)> {
    let columns = columns(section, tier, None);
    let mark = columns.mark.unwrap_or(columns.label);
    match section {
        SectionId::Virtues => {
            let mut heads = vec![(columns.label, text::HEAD_VIRTUE)];
            if let Some(mantra) = columns.mantra {
                heads.push((mantra, text::HEAD_MANTRA));
            }
            heads.push((mark, text::HEAD_QUEST));
            heads
        }
        SectionId::Dungeons => vec![
            (columns.label, text::HEAD_DUNGEON),
            (mark, text::HEAD_ENTRANCE),
        ],
        SectionId::Shadowlords => vec![
            (columns.label, text::HEAD_NAME),
            (mark, text::HEAD_STATE),
            (columns.note.unwrap_or(mark), text::HEAD_SHARD),
        ],
        _ => Vec::new(),
    }
}

/// The phase a virtue row's marker stands for.
fn phase_of(mark: StatusMark) -> Option<ShrinePhase> {
    [
        ShrinePhase::NotStarted,
        ShrinePhase::Ordained,
        ShrinePhase::CodexRead,
        ShrinePhase::Complete,
    ]
    .into_iter()
    .find(|phase| StatusMark::for_phase(*phase) == mark)
}

/// Whether a virtue row's shrine is in ruin.
fn ruined(row: &JournalRow) -> bool {
    row.note
        .as_ref()
        .is_some_and(|(mark, _)| *mark == StatusMark::ALERT)
}

/// The cells of one body row on grid row `y`.
fn row_cells(
    view: &PageView<'_>,
    section: SectionId,
    row: &JournalRow,
    y: u8,
    selected: bool,
) -> Vec<Cell> {
    let tier = view.tier();
    let columns = columns(section, tier, Some(row));
    let mut cells = Vec::new();
    if selected {
        cells.push(Cell::new(TEXT_COLUMN, y, " ".repeat(TEXT_CELLS), WHITE).on(Paper::Bar));
    }
    let ink = if selected { WHITE } else { BODY };
    let label = match row.key {
        RowKey::Shadowlord(slot) if row.label.as_str() == text::shard_label(slot as usize) => {
            text::shard_cell_label(slot as usize)
        }
        _ => row.label.as_str(),
    };
    cells.push(Cell::new(columns.label, y, label, ink));
    if let Some(column) = columns.mark
        && row.mark.glyph != ' '
    {
        cells.push(Cell::new(column, y, row.mark.glyph, ega(row.mark.colour)));
    }
    let suffix = if section == SectionId::Virtues && ruined(row) {
        Some(StatusMark::ALERT)
    } else {
        row.suffix
    };
    if let (Some(column), Some(suffix)) = (columns.mark, suffix) {
        cells.push(Cell::new(column + 1, y, suffix.glyph, ega(suffix.colour)));
    }
    if let Some(column) = columns.value {
        let (value, value_ink) = match section {
            // A ruined shrine reads Ruined, in red; its marker keeps the
            // phase and the details give it in words (5.2.6).
            SectionId::Virtues if ruined(row) => (text::RUINED, ega(StatusMark::ALERT.colour)),
            SectionId::Virtues if columns.mantra.is_some() => (
                phase_of(row.mark)
                    .map(text::phase_cell)
                    .unwrap_or_else(|| row.value.as_str()),
                BODY,
            ),
            _ => (row.value.as_str(), BODY),
        };
        if !value.is_empty() {
            cells.push(Cell::new(column, y, value, value_ink));
        }
    }
    if let (Some(column), Some((_, note))) = (columns.note, &row.note) {
        let note = match note.as_str() {
            text::SHARD_CARRIED => text::SHARD_CARRIED_CELL,
            text::SHARD_DESTROYED => text::SHARD_DESTROYED_CELL,
            other => other,
        };
        cells.push(Cell::new(column, y, note, BODY));
    }
    if let (Some(column), Some(secret)) = (columns.mantra, row.secret)
        && view.mantras == MantraDisplay::Reveal
    {
        cells.push(answer_cell(
            column,
            y,
            secret,
            view.revealed.contains(&row.key),
        ));
    }
    cells
}

/// A tier 4 answer: `[hidden]` dark grey on blue, or the answer in white.
fn answer_cell(column: u8, y: u8, secret: &str, revealed: bool) -> Cell {
    if revealed {
        Cell::new(column, y, secret, WHITE)
    } else {
        Cell::new(column, y, text::HIDDEN, DARK_GREY).on(Paper::Mask)
    }
}

/// `text` broken at spaces into at most `lines` lines of `width` cells; a
/// cut ends in `..`.
pub fn wrap(text: &str, width: usize, lines: usize) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for word in text.split_whitespace() {
        let word: String = word.chars().take(width).collect();
        let fits = out
            .last()
            .is_some_and(|line| line.chars().count() + 1 + word.chars().count() <= width);
        if let (true, Some(line)) = (fits, out.last_mut()) {
            line.push(' ');
            line.push_str(&word);
        } else if out.len() < lines {
            out.push(word);
        } else {
            if let Some(line) = out.last_mut() {
                while line.chars().count() + 2 > width {
                    match line.rfind(' ') {
                        Some(at) => line.truncate(at),
                        None => *line = line.chars().take(width.saturating_sub(2)).collect(),
                    }
                }
                line.push_str("..");
            }
            return out;
        }
    }
    out
}

/// Every text cell of the page below the title band and above the key help
/// band: the header, the section switcher, the headings, the visible rows
/// and the selected row's details.
pub fn page_cells(view: &PageView<'_>) -> Vec<Cell> {
    let mut cells = Vec::new();
    let Some(model) = view.model else {
        return cells;
    };
    cells.push(Cell::new(
        TEXT_COLUMN,
        HEADER_ROW,
        model.header.as_str(),
        BROWN,
    ));
    let Some(section) = view.section else {
        return cells;
    };
    let tier = model.tier;

    // The switcher: no count, because the number of sections changes with
    // what the player has found (5.2.4).
    let title = section.id.title();
    cells.push(Cell::new(TEXT_COLUMN, SECTION_ROW, GLYPH_LEFT, BROWN));
    cells.push(Cell::new(TEXT_COLUMN + 2, SECTION_ROW, title, WHITE));
    cells.push(Cell::new(
        TEXT_COLUMN + 3 + title.chars().count() as u8,
        SECTION_ROW,
        GLYPH_RIGHT,
        BROWN,
    ));
    if section.rows.len() > VISIBLE_ROWS {
        let more_above = view.scroll > 0;
        let more_below = view.scroll + VISIBLE_ROWS < section.rows.len();
        for (column, glyph, more) in [
            (LAST_TEXT_COLUMN - 1, GLYPH_UP, more_above),
            (LAST_TEXT_COLUMN, GLYPH_DOWN, more_below),
        ] {
            let ink = if more { BROWN } else { DARK_GREY };
            cells.push(Cell::new(column, SECTION_ROW, glyph, ink));
        }
    }
    for (column, head) in headings(section.id, tier) {
        cells.push(Cell::new(column, HEADING_ROW, head, WHITE));
    }

    for (offset, row) in section
        .rows
        .iter()
        .enumerate()
        .skip(view.scroll)
        .take(VISIBLE_ROWS)
    {
        let y = ROWS_FIRST_ROW + (offset - view.scroll) as u8;
        cells.extend(row_cells(view, section.id, row, y, offset == view.cursor));
    }

    if let Some(row) = view.selected() {
        let joined = row
            .detail
            .iter()
            .map(|line| line.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        // A Word of Power or a flame is masked on the last detail row; a
        // mantra is in its own column.
        let answer = (section.id != SectionId::Virtues
            && masked_answer(section.id, row, view.mantras))
        .then_some(row.secret)
        .flatten();
        let room = if answer.is_some() { 1 } else { 2 };
        for (index, line) in wrap(&joined, TEXT_CELLS, room).into_iter().enumerate() {
            cells.push(Cell::new(
                TEXT_COLUMN,
                DETAIL_FIRST_ROW + index as u8,
                line,
                BODY,
            ));
        }
        if let Some(secret) = answer {
            let label = format!("{}: ", text::secret_label(section.id));
            let column = TEXT_COLUMN + label.chars().count() as u8;
            cells.push(Cell::new(TEXT_COLUMN, DETAIL_LAST_ROW, label, BODY));
            cells.push(answer_cell(
                column,
                DETAIL_LAST_ROW,
                secret,
                view.revealed.contains(&row.key),
            ));
        }
    }
    cells
}

/// The key help: Enter while the selected row has an answer still masked.
pub fn key_help(view: &PageView<'_>) -> &'static str {
    let maskable = view
        .section
        .zip(view.selected())
        .is_some_and(|(section, row)| {
            masked_answer(section.id, row, view.mantras) && !view.revealed.contains(&row.key)
        });
    if maskable { KEY_HELP_REVEAL } else { KEY_HELP }
}

/// Paints the whole page into a 320x200 RGBA buffer: black, the game's
/// frame (blue bands, white rules, rounded corners, divider bands that
/// join the side bands), the title tab, the key help tab and
/// [`page_cells`].
pub fn paint_page(rgba: &mut [u8], font: &FixedCellFont, view: &PageView<'_>) {
    debug_assert_eq!(rgba.len(), PAGE_WIDTH * PAGE_HEIGHT * 4);
    fill(rgba, 0, 0, PAGE_WIDTH - 1, PAGE_HEIGHT - 1, BLACK);
    paint_frame(rgba);

    let spoilers = view.model.is_some_and(|model| model.spoilers);
    let mut title = vec![(text::TITLE, WHITE)];
    if spoilers {
        title.push((" ", WHITE));
        title.push((text::SPOILERS, BROWN));
    }
    paint_band_tab(rgba, font, TITLE_ROW, &title);
    paint_band_tab(rgba, font, KEY_HELP_ROW, &[(key_help(view), WHITE)]);

    for cell in page_cells(view) {
        let x0 = cell.column as usize * CH_CELL_SIDE;
        let y0 = cell.row as usize * CH_CELL_SIDE;
        let x1 = (cell.last_column() + 1) * CH_CELL_SIDE - 1;
        let half = CH_CELL_SIDE / 2;
        match cell.paper {
            Some(Paper::Bar) => fill(
                rgba,
                x0 - half,
                y0,
                x1 + half,
                y0 + CH_CELL_SIDE - 1,
                MASK_PAPER,
            ),
            Some(Paper::Mask) => fill(rgba, x0, y0, x1, y0 + CH_CELL_SIDE - 2, MASK_PAPER),
            None => {}
        }
        put_text(rgba, font, &cell.text, cell.column, cell.row, cell.ink);
    }
}

/// The blue bands, the rounded outer corners and the white rules. A
/// divider band joins the side bands as the stats panel's divider does in
/// the gameplay frame: the side rules stop at its rules, which run from
/// side rule to side rule.
fn paint_frame(rgba: &mut [u8]) {
    let right = PAGE_WIDTH - 1;
    let bottom = PAGE_HEIGHT - 1;
    // Bands: top, bottom, left, right.
    fill(rgba, 0, 0, right, RULE_TOP_Y - 1, BAND);
    fill(rgba, 0, RULE_BOTTOM_Y + 1, right, bottom, BAND);
    fill(rgba, 0, 0, RULE_LEFT_X - 1, bottom, BAND);
    fill(rgba, RULE_RIGHT_X + 1, 0, right, bottom, BAND);
    for row in [UPPER_DIVIDER_ROW, LOWER_DIVIDER_ROW] {
        let (top, under) = divider_rules(row);
        fill(rgba, RULE_LEFT_X, top + 1, RULE_RIGHT_X, under - 1, BAND);
    }
    // The bevel: each of the first rows from an outer edge starts its fill
    // at the profile's column, at every corner.
    for (from_edge, &start) in CHROME_CORNER_PROFILE.iter().enumerate() {
        let start = start as usize;
        if start == 0 {
            continue;
        }
        for y in [from_edge, bottom - from_edge] {
            fill(rgba, 0, y, start - 1, y, BLACK);
            fill(rgba, right + 1 - start, y, right, y, BLACK);
        }
    }
    // The rules.
    fill(
        rgba,
        RULE_LEFT_X,
        RULE_TOP_Y,
        RULE_RIGHT_X,
        RULE_TOP_Y,
        WHITE,
    );
    fill(
        rgba,
        RULE_LEFT_X,
        RULE_BOTTOM_Y,
        RULE_RIGHT_X,
        RULE_BOTTOM_Y,
        WHITE,
    );
    let mut from = RULE_TOP_Y;
    for row in [UPPER_DIVIDER_ROW, LOWER_DIVIDER_ROW] {
        let (top, under) = divider_rules(row);
        fill(rgba, RULE_LEFT_X, top, RULE_RIGHT_X, top, WHITE);
        fill(rgba, RULE_LEFT_X, under, RULE_RIGHT_X, under, WHITE);
        for x in [RULE_LEFT_X, RULE_RIGHT_X] {
            fill(rgba, x, from, x, top, WHITE);
        }
        from = under;
    }
    for x in [RULE_LEFT_X, RULE_RIGHT_X] {
        fill(rgba, x, from, x, RULE_BOTTOM_Y, WHITE);
    }
}

/// A divider band's two rules: the first and last pixel rows of its cell
/// row, as the stats panel's band at row 7 has them (y = 56 and 63).
pub const fn divider_rules(row: u8) -> (usize, usize) {
    let top = row as usize * CH_CELL_SIDE;
    (top, top + CH_CELL_SIDE - 1)
}

/// The cells a band tab takes: the left cap's column, the first text
/// column and the right cap's column. The tab is centred on the page.
pub fn band_tab_columns(cells: usize) -> (u8, u8, u8) {
    let cells = cells.min(PAGE_WIDTH / CH_CELL_SIDE - 4);
    let span = cells + 2;
    let left_cap = ((PAGE_WIDTH / CH_CELL_SIDE - span) / 2) as u8;
    (left_cap, left_cap + 1, left_cap + 1 + cells as u8)
}

/// A label tab punched through a band, as the gameplay frame's
/// `>Calm Winds<`: the gap is black over the whole cell row, the band's
/// rule stops at the caps, and the caps' strokes join it.
fn paint_band_tab(rgba: &mut [u8], font: &FixedCellFont, row: u8, parts: &[(&str, Rgb)]) {
    let cells: usize = parts.iter().map(|(text, _)| text.chars().count()).sum();
    let (left_cap, first, right_cap) = band_tab_columns(cells);
    let y0 = row as usize * CH_CELL_SIDE;
    let x0 = left_cap as usize * CH_CELL_SIDE;
    let x1 = right_cap as usize * CH_CELL_SIDE + CH_CELL_SIDE - 1;
    fill(rgba, x0, y0, x1, y0 + CH_CELL_SIDE - 1, BLACK);
    let rule_y = if row == TITLE_ROW {
        RULE_TOP_Y
    } else {
        RULE_BOTTOM_Y
    };
    fill(rgba, RULE_LEFT_X, rule_y, x0, rule_y, WHITE);
    fill(rgba, x1, rule_y, RULE_RIGHT_X, rule_y, WHITE);
    for (direction, column) in [
        (RibbonCapDirection::Right, left_cap),
        (RibbonCapDirection::Left, right_cap),
    ] {
        paint_ribbon_cap(
            rgba,
            PAGE_WIDTH,
            PAGE_HEIGHT,
            font,
            direction,
            column,
            row,
            ChromePalette::EGA,
        );
    }
    let mut column = first;
    for (text, colour) in parts {
        let room = right_cap.saturating_sub(column) as usize;
        let shown: String = text.chars().take(room).collect();
        put_text(rgba, font, &shown, column, row, *colour);
        column += shown.chars().count() as u8;
    }
}

/// IBM.CH text from `column`, one glyph per cell with a transparent
/// background, clipped at the frame's right rule.
fn put_text(rgba: &mut [u8], font: &FixedCellFont, text: &str, column: u8, row: u8, ink: Rgb) {
    let last = RULE_RIGHT_X / CH_CELL_SIDE;
    for (offset, ch) in text.chars().enumerate() {
        let cell = column as usize + offset;
        if cell >= last {
            break;
        }
        let code = (ch as u32 as u8) & 0x7f;
        for glyph_y in 0..CH_CELL_SIDE {
            let bits = font.glyph_row(code, glyph_y).unwrap_or(0);
            let y = row as usize * CH_CELL_SIDE + glyph_y;
            if y >= PAGE_HEIGHT {
                return;
            }
            for glyph_x in 0..CH_CELL_SIDE {
                if bits & (0x80 >> glyph_x) != 0 {
                    put_pixel(rgba, cell * CH_CELL_SIDE + glyph_x, y, ink);
                }
            }
        }
    }
}

fn put_pixel(rgba: &mut [u8], x: usize, y: usize, ink: Rgb) {
    let offset = (y * PAGE_WIDTH + x) * 4;
    rgba[offset..offset + 4].copy_from_slice(&[ink[0], ink[1], ink[2], 0xff]);
}

/// An inclusive pixel rectangle in one colour.
fn fill(rgba: &mut [u8], x0: usize, y0: usize, x1: usize, y1: usize, ink: Rgb) {
    for y in y0..=y1.min(PAGE_HEIGHT - 1) {
        for x in x0..=x1.min(PAGE_WIDTH - 1) {
            put_pixel(rgba, x, y, ink);
        }
    }
}

mod panel;
pub use panel::{
    JournalPanelPageOpen, JournalPanelProvider, PANEL_SEALS_OPEN, PANEL_TITLE, PANEL_VANQUISHED,
    SidePanelModel, panel_side,
};

#[cfg(test)]
mod content_tests;
#[cfg(test)]
mod panel_tests;
#[cfg(test)]
mod review_tests;
#[cfg(test)]
mod tests;

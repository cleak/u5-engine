//! The quest journal page in the Bevy shell (Phase B design 5.2.2, 5.2.4,
//! 5.3.4; build plan M5-03).
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
//! **Tabs (the M6 seam).** Chapter 8 turns the page into several tabs.
//! [`QuestJournalUi`] keeps a tab list that holds only [`JournalTab::Quests`]
//! here; no tab strip is drawn while there is one tab.

use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use u5_runtime::enhancements::{JournalSettings, Toggle};
use u5_runtime::gameplay_chrome::{
    CHROME_CORNER_PROFILE, ChromePalette, RibbonCapDirection, paint_ribbon_cap,
};
use u5_runtime::quest_journal::{JournalModel, JournalSection, QuestSnapshot, derive, text};
use u5_runtime::{CH_CELL_SIDE, EGA_PALETTE_RGB, FixedCellFont, STOCK_EGA_BROWN};

use crate::enh::input::{EnhAction, EnhActionEvent, OverlayKey, menu_overlay_on};
use crate::enh::overlay::{OverlayId, OverlayRequest, OverlayState, apply_overlay_requests};
use crate::enh::side::FrameImageSize;
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
/// Rows 3-20: the section body.
pub const BODY_FIRST_ROW: u8 = 3;
pub const BODY_LAST_ROW: u8 = 20;
/// The band between the body and the details.
pub const LOWER_DIVIDER_ROW: u8 = 21;
/// Rows 22-23: the selected row's details (M5-04).
pub const DETAIL_FIRST_ROW: u8 = 22;
/// Row 24: the bottom band, with the key help tab.
pub const KEY_HELP_ROW: u8 = 24;

/// Text starts one cell in from the frame's left rule.
const TEXT_COLUMN: u8 = 2;
/// A body row's label, its marker and its value.
const LABEL_COLUMN: u8 = 3;
const MARK_COLUMN: u8 = 17;
const VALUE_COLUMN: u8 = 19;

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
pub const BROWN: Rgb = STOCK_EGA_BROWN;

/// The key help while only the shell exists: M5-04 adds the section and
/// row keys as it adds them.
pub const KEY_HELP: &str = "Esc close";

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
    /// The section shown, an index into the model's sections.
    section: usize,
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
            section: 0,
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
        .is_some_and(|keys| keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]));
    let mut close = false;
    for OverlayKey(key) in keys.read() {
        if !open {
            continue;
        }
        match key {
            KeyCode::Escape => close = true,
            KeyCode::KeyJ if ctrl => close = true,
            _ => {}
        }
    }
    if close {
        requests.write(OverlayRequest::Close(OverlayId::Journal));
        ui.dirty = true;
    }
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
        ui.last_model = Some(model);
        ui.dirty = true;
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
    let section = ui
        .last_model
        .as_ref()
        .and_then(|model| model.sections.get(ui.section));
    paint_page(
        rgba,
        &visual.text_font,
        &PageView {
            model: ui.last_model.as_ref(),
            section,
            tab_strip: ui.tabs.len() > 1,
        },
    );
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
    /// Chapter 8's strip, drawn only with more than one tab. M5 never sets
    /// it.
    pub tab_strip: bool,
}

/// Paints the whole page into a 320x200 RGBA buffer: black, the game's
/// frame (blue bands, white rules, rounded corners, divider bands that
/// join the side bands), the title tab, the summary header, the section
/// body and the key help tab.
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
    paint_band_tab(rgba, font, KEY_HELP_ROW, &[(KEY_HELP, WHITE)]);

    let Some(model) = view.model else {
        return;
    };
    put_text(rgba, font, &model.header, TEXT_COLUMN, HEADER_ROW, BROWN);
    let Some(section) = view.section else {
        return;
    };
    put_text(
        rgba,
        font,
        section.id.title(),
        TEXT_COLUMN,
        BODY_FIRST_ROW,
        WHITE,
    );
    let rows = (BODY_LAST_ROW - BODY_FIRST_ROW) as usize;
    for (index, row) in section.rows.iter().take(rows).enumerate() {
        let y = BODY_FIRST_ROW + 1 + index as u8;
        put_text(rgba, font, row.label.as_str(), LABEL_COLUMN, y, WHITE);
        put_char(
            rgba,
            font,
            row.mark.glyph,
            MARK_COLUMN,
            y,
            ega(row.mark.colour),
        );
        if let Some(suffix) = row.suffix {
            put_char(
                rgba,
                font,
                suffix.glyph,
                MARK_COLUMN + 1,
                y,
                ega(suffix.colour),
            );
        }
        let value = row.value.as_str();
        put_text(rgba, font, value, VALUE_COLUMN, y, BODY);
        if let Some((mark, note)) = &row.note {
            let column = match value.chars().count() {
                0 => VALUE_COLUMN,
                cells => VALUE_COLUMN + cells as u8 + 2,
            };
            put_char(rgba, font, mark.glyph, column, y, ega(mark.colour));
            put_text(rgba, font, note.as_str(), column + 2, y, ega(mark.colour));
        }
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

fn put_char(rgba: &mut [u8], font: &FixedCellFont, glyph: char, column: u8, row: u8, ink: Rgb) {
    if glyph != ' ' {
        let mut buffer = [0u8; 4];
        put_text(rgba, font, glyph.encode_utf8(&mut buffer), column, row, ink);
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

#[cfg(test)]
mod review_tests;
#[cfg(test)]
mod tests;

//! The windowed first-run setup wizard, as a Bevy plugin.
//!
//! Screens: welcome; the folders discovery found, each with its validation
//! status; "Choose folder..." through the host's native picker; a review of
//! the chosen folder that lists any missing files in plain language; "How to
//! get the files"; and a final confirmation that hands over to the game.
//!
//! The Enhancements page (Phase B design 1.2.2), opened from the welcome
//! screen or on its own ([`run_settings_wizard`], `u5-engine --settings`),
//! lists the rows [`WizardHost::settings_rows`] describes and stores the
//! changed ones through [`WizardHost::save_settings`]. It knows rows, not the
//! enhancement catalogue, so a feature added later appears without new UI
//! code.
//!
//! The wizard knows nothing about desktops: everything platform-specific
//! comes through the [`WizardHost`] trait. It uses plain Bevy UI with the
//! default font (it runs precisely when the game's own font files may be
//! missing) in the game's 16-colour palette.

pub mod model;

use std::sync::{Arc, Mutex};

use bevy::app::AppExit;
use bevy::prelude::*;
use bevy::window::WindowCloseRequested;
use u5_gamedata::{Discovered, ValidationReport};

pub use model::{
    Action, CommitSummary, Confirm, FocusItem, MixLabel, PresetChoice, RowKind, RowRole, RowValue,
    Screen, SettingRow, SettingsChanges, SettingsPage, SettingsRows, WizardHost, WizardModel,
    WizardOutcome,
};

// The EGA palette entries the game itself draws its menus in.
const EGA_BLACK: Color = Color::srgb(0.0, 0.0, 0.0);
const EGA_BLUE: Color = Color::srgb(0.0, 0.0, 0.667);
const EGA_CYAN: Color = Color::srgb(0.0, 0.667, 0.667);
const EGA_LIGHT_GREY: Color = Color::srgb(0.667, 0.667, 0.667);
const EGA_DARK_GREY: Color = Color::srgb(0.333, 0.333, 0.333);
const EGA_LIGHT_GREEN: Color = Color::srgb(0.333, 1.0, 0.333);
const EGA_LIGHT_RED: Color = Color::srgb(1.0, 0.333, 0.333);
const EGA_YELLOW: Color = Color::srgb(1.0, 1.0, 0.333);
const EGA_WHITE: Color = Color::srgb(1.0, 1.0, 1.0);

/// The host, shared with the systems.
#[derive(Resource, Clone)]
pub struct WizardHostResource(pub Arc<dyn WizardHost>);

/// The wizard's state.
#[derive(Resource)]
pub struct WizardState {
    pub model: WizardModel,
    /// The UI needs rebuilding.
    pub dirty: bool,
}

/// Where the outcome is written when the wizard finishes.
#[derive(Resource, Clone, Default)]
pub struct WizardResult(pub Arc<Mutex<Option<WizardOutcome>>>);

/// A clickable wizard button.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct WizardButton(pub Action);

/// Root of the wizard UI (despawned and rebuilt on every change).
#[derive(Component)]
pub struct WizardRoot;

/// A clickable Enhancements-page row. Its colour shows the keyboard focus,
/// so the hover colouring leaves it alone.
#[derive(Component)]
pub struct WizardRowButton;

/// State handling only: input, picker polling, exit. No rendering, so it
/// runs under `MinimalPlugins` in tests.
pub struct WizardCorePlugin;

impl Plugin for WizardCorePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WizardResult>()
            .add_event::<WindowCloseRequested>()
            .add_systems(
                Update,
                (
                    handle_buttons,
                    handle_keys,
                    poll_picker,
                    cancel_on_close,
                    finish_when_done,
                )
                    .chain(),
            );
    }
}

/// The full wizard: [`WizardCorePlugin`] plus the UI.
pub struct SetupWizardPlugin;

impl Plugin for SetupWizardPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(WizardCorePlugin)
            .insert_resource(ClearColor(EGA_BLACK))
            .add_systems(Startup, spawn_camera)
            .add_systems(
                Update,
                (rebuild_ui, button_hover).chain().after(finish_when_done),
            );
    }
}

/// Run the wizard in its own window and return how it ended.
///
/// Bevy's windowing can only be started once per process, so a frontend
/// that will open the game window afterwards should run this in a separate
/// process (the desktop frontend does).
pub fn run_setup_wizard(
    host: Arc<dyn WizardHost>,
    reason: String,
    candidates: Vec<Discovered>,
) -> WizardOutcome {
    let model = WizardModel::new(reason, candidates).offer_settings(host.as_ref());
    run_wizard_window(host, model)
}

/// Run only the Enhancements page in its own window (`u5-engine
/// --settings`). Ends with [`WizardOutcome::SettingsClosed`] after Save or
/// Cancel, or [`WizardOutcome::Cancelled`] when the window is closed. A host
/// without settings opens no window.
pub fn run_settings_wizard(host: Arc<dyn WizardHost>) -> WizardOutcome {
    let model = WizardModel::settings_only(host.as_ref());
    if let Some(outcome) = model.outcome.clone() {
        return outcome;
    }
    run_wizard_window(host, model)
}

fn run_wizard_window(host: Arc<dyn WizardHost>, model: WizardModel) -> WizardOutcome {
    let result = WizardResult::default();
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Ultima V - Setup".into(),
                resolution: (960.0_f32, 720.0_f32).into(),
                resizable: true,
                ..default()
            }),
            ..default()
        }))
        .insert_resource(WizardHostResource(host))
        .insert_resource(WizardState { model, dirty: true })
        .insert_resource(result.clone())
        .add_plugins(SetupWizardPlugin)
        .run();
    let outcome = result.0.lock().ok().and_then(|mut slot| slot.take());
    outcome.unwrap_or(WizardOutcome::Cancelled)
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn handle_buttons(
    buttons: Query<(&Interaction, &WizardButton), Changed<Interaction>>,
    host: Res<WizardHostResource>,
    mut state: ResMut<WizardState>,
) {
    let mut pressed = None;
    for (interaction, button) in &buttons {
        if *interaction == Interaction::Pressed {
            pressed = Some(button.0.clone());
        }
    }
    if let Some(action) = pressed
        && state.model.apply(action, host.0.as_ref())
    {
        state.dirty = true;
    }
}

fn handle_keys(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    host: Res<WizardHostResource>,
    mut state: ResMut<WizardState>,
) {
    let Some(keys) = keys else {
        return;
    };
    let action = if state.model.screen == Screen::Enhancements {
        settings_key_action(&keys, state.model.settings.as_ref())
    } else if keys.just_pressed(KeyCode::Escape) {
        Some(match state.model.screen {
            Screen::Welcome => Action::Quit,
            _ => Action::Back,
        })
    } else if keys.just_pressed(KeyCode::Enter) {
        primary_action(&state.model.screen)
    } else {
        None
    };
    if let Some(action) = action
        && state.model.apply(action, host.0.as_ref())
    {
        state.dirty = true;
    }
}

/// The Enhancements page's keys (design 1.2.2): Up/Down and Tab move
/// between rows, Space toggles, Enter saves, Escape cancels. While a
/// question is open, Enter or Y answers yes, N answers no, and Escape
/// cancels it.
fn settings_key_action(keys: &ButtonInput<KeyCode>, page: Option<&SettingsPage>) -> Option<Action> {
    let pressed = |code| keys.just_pressed(code);
    if page.is_some_and(|page| page.confirm.is_some()) {
        return if pressed(KeyCode::Enter) || pressed(KeyCode::KeyY) {
            Some(Action::ConfirmYes)
        } else if pressed(KeyCode::KeyN) {
            Some(Action::ConfirmNo)
        } else if pressed(KeyCode::Escape) {
            Some(Action::Back)
        } else {
            None
        };
    }
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if pressed(KeyCode::Escape) {
        Some(Action::Back)
    } else if pressed(KeyCode::Enter) {
        Some(Action::SaveSettings)
    } else if pressed(KeyCode::Space) {
        Some(Action::ActivateFocus)
    } else if pressed(KeyCode::ArrowDown) || (pressed(KeyCode::Tab) && !shift) {
        Some(Action::FocusNext)
    } else if pressed(KeyCode::ArrowUp) || (pressed(KeyCode::Tab) && shift) {
        Some(Action::FocusPrev)
    } else {
        None
    }
}

/// What Enter does on each screen.
pub fn primary_action(screen: &Screen) -> Option<Action> {
    match screen {
        Screen::Welcome => Some(Action::Continue),
        Screen::Review { report, .. } if report.is_usable() => Some(Action::UseFolder),
        Screen::Done(_) => Some(Action::Start),
        Screen::Help => Some(Action::Back),
        Screen::Enhancements => Some(Action::SaveSettings),
        _ => None,
    }
}

fn poll_picker(host: Res<WizardHostResource>, mut state: ResMut<WizardState>) {
    if state.model.poll(host.0.as_ref()) {
        state.dirty = true;
    }
}

fn finish_when_done(
    state: Res<WizardState>,
    result: Res<WizardResult>,
    mut exit: EventWriter<AppExit>,
) {
    if let Some(outcome) = &state.model.outcome
        && let Ok(mut slot) = result.0.lock()
        && slot.is_none()
    {
        *slot = Some(outcome.clone());
        exit.write(AppExit::Success);
    }
}

fn cancel_on_close(
    mut closed: EventReader<WindowCloseRequested>,
    host: Res<WizardHostResource>,
    mut state: ResMut<WizardState>,
) {
    if closed.read().next().is_some() {
        state.model.apply(Action::Quit, host.0.as_ref());
    }
}

type HoverQuery<'w, 's> = Query<
    'w,
    's,
    (&'static Interaction, &'static mut BackgroundColor),
    (
        Changed<Interaction>,
        With<WizardButton>,
        Without<WizardRowButton>,
    ),
>;

fn button_hover(mut buttons: HoverQuery) {
    for (interaction, mut background) in &mut buttons {
        background.0 = match interaction {
            Interaction::Pressed | Interaction::Hovered => EGA_CYAN,
            Interaction::None => EGA_BLUE,
        };
    }
}

fn rebuild_ui(
    mut commands: Commands,
    mut state: ResMut<WizardState>,
    host: Res<WizardHostResource>,
    roots: Query<Entity, With<WizardRoot>>,
) {
    if !state.dirty {
        return;
    }
    state.dirty = false;
    for root in &roots {
        commands.entity(root).despawn();
    }
    let page = page_for(&state.model, host.0.as_ref());
    spawn_page(&mut commands, page);
}

/// A screen's content, separated from spawning so it can be tested.
#[derive(Clone, Debug, PartialEq)]
pub struct Page {
    pub title: String,
    pub lines: Vec<(String, LineStyle)>,
    /// Buttons that each take a full row (candidate folders).
    pub list: Vec<(String, Action)>,
    /// Setting rows (the Enhancements page), flowing into columns.
    pub rows: Vec<PageRow>,
    pub buttons: Vec<(String, Action)>,
}

/// One line of the Enhancements page: a group heading, the preset row, or
/// a setting.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PageRow {
    pub text: String,
    pub style: RowStyle,
    /// What clicking it does; `None` for headings.
    pub action: Option<Action>,
    /// The keyboard focus is on it.
    pub focused: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowStyle {
    Heading,
    Setting,
    /// A cheat while cheats are not allowed.
    Greyed,
}

impl RowStyle {
    fn color(self) -> Color {
        match self {
            Self::Heading => EGA_CYAN,
            Self::Setting => EGA_LIGHT_GREY,
            Self::Greyed => EGA_DARK_GREY,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineStyle {
    Body,
    Good,
    Bad,
    Dim,
}

impl LineStyle {
    fn color(self) -> Color {
        match self {
            Self::Body => EGA_LIGHT_GREY,
            Self::Good => EGA_LIGHT_GREEN,
            Self::Bad => EGA_LIGHT_RED,
            Self::Dim => EGA_DARK_GREY,
        }
    }
}

/// Lay out the current screen.
pub fn page_for(model: &WizardModel, host: &dyn WizardHost) -> Page {
    let line = |text: &str, style| (text.to_string(), style);
    match &model.screen {
        Screen::Welcome => {
            let mut buttons = vec![
                ("Continue".to_string(), Action::Continue),
                ("How to get the files".to_string(), Action::ShowHelp),
            ];
            if model.offers_settings {
                buttons.push(("Enhancements...".to_string(), Action::OpenSettings));
            }
            buttons.push(("Quit".to_string(), Action::Quit));
            let mut page = Page {
                title: "Welcome".to_string(),
                lines: vec![
                    line(
                        "This engine plays Ultima V: Warriors of Destiny using the data files \
                     from your own copy of the game.",
                        LineStyle::Body,
                    ),
                    line(&model.reason, LineStyle::Body),
                    line(
                        "The next screen shows the folders found on this computer. You can \
                     also choose a folder yourself. Your files are only read: setup copies \
                     them into the engine's own data folder.",
                        LineStyle::Body,
                    ),
                ],
                list: Vec::new(),
                rows: Vec::new(),
                buttons,
            };
            if let Some(notice) = &model.notice {
                page.lines.push(line(notice, LineStyle::Good));
            }
            page
        }
        Screen::Enhancements => match &model.settings {
            Some(settings) => settings_page(settings),
            None => Page {
                title: "Enhancements".to_string(),
                lines: Vec::new(),
                list: Vec::new(),
                rows: Vec::new(),
                buttons: vec![("Back".to_string(), Action::Back)],
            },
        },
        Screen::Candidates => {
            let mut lines = Vec::new();
            if model.candidates.is_empty() {
                lines.push(line(
                    "No game folders were found automatically. Choose the folder that \
                     contains the game files (such as BRIT.DAT), or read how to get them.",
                    LineStyle::Body,
                ));
            } else {
                lines.push(line(
                    "These folders look like Ultima V. Pick one to check it:",
                    LineStyle::Body,
                ));
            }
            if let Some(notice) = &model.notice {
                lines.push(line(notice, LineStyle::Dim));
            }
            let list = model
                .candidates
                .iter()
                .enumerate()
                .map(|(index, found)| {
                    (
                        format!(
                            "{}\n  {} - {}",
                            found.data_root.display(),
                            found.candidate.source,
                            found.report.status_line()
                        ),
                        Action::SelectCandidate(index),
                    )
                })
                .collect();
            Page {
                title: "Find the game files".to_string(),
                lines,
                list,
                rows: Vec::new(),
                buttons: vec![
                    ("Choose folder...".to_string(), Action::ChooseFolder),
                    ("Search again".to_string(), Action::Rescan),
                    ("How to get the files".to_string(), Action::ShowHelp),
                    ("Quit".to_string(), Action::Quit),
                ],
            }
        }
        Screen::Picking => Page {
            title: "Choose a folder".to_string(),
            lines: vec![line(
                "Choose the game folder in the window that opened.",
                LineStyle::Body,
            )],
            list: Vec::new(),
            rows: Vec::new(),
            buttons: Vec::new(),
        },
        Screen::Review {
            path,
            report,
            error,
        } => {
            let mut lines = vec![line(&format!("{}", path.display()), LineStyle::Dim)];
            lines.extend(review_lines(report));
            if let Some(error) = error {
                lines.push(line(&format!("Setup failed: {error}"), LineStyle::Bad));
            }
            let mut buttons = Vec::new();
            if report.is_usable() {
                buttons.push(("Use this folder".to_string(), Action::UseFolder));
            }
            buttons.push(("Choose another folder...".to_string(), Action::ChooseFolder));
            buttons.push(("Back".to_string(), Action::Back));
            if !report.is_usable() {
                buttons.push(("How to get the files".to_string(), Action::ShowHelp));
            }
            Page {
                title: if report.is_usable() {
                    "Game files found".to_string()
                } else {
                    "This folder cannot be used".to_string()
                },
                lines,
                list: Vec::new(),
                rows: Vec::new(),
                buttons,
            }
        }
        Screen::Help => Page {
            title: "How to get the files".to_string(),
            lines: vec![line(&host.help_text(), LineStyle::Body)],
            list: Vec::new(),
            rows: Vec::new(),
            buttons: vec![("Back".to_string(), Action::Back)],
        },
        Screen::Done(summary) => Page {
            title: "Ready".to_string(),
            lines: vec![
                line(&summary.message, LineStyle::Good),
                line(
                    "Your choice is saved. Run the program with --setup to change it.",
                    LineStyle::Body,
                ),
            ],
            list: Vec::new(),
            rows: Vec::new(),
            buttons: vec![
                ("Start the game".to_string(), Action::Start),
                ("Quit".to_string(), Action::Quit),
            ],
        },
    }
}

/// The label of the Cheats group, which says what it does to a game.
pub const CHEATS_GROUP: &str = "Cheats (changes your game)";

/// The Enhancements page (design 1.2.2), or the question it is waiting on.
pub fn settings_page(page: &SettingsPage) -> Page {
    if let Some(confirm) = &page.confirm {
        return confirm_page(page, confirm);
    }
    let focused = page.focused();
    let mut lines = Vec::new();
    for note in &page.notes {
        lines.push((note.clone(), LineStyle::Bad));
    }
    if page.locked {
        lines.push((
            "The parity lock is on for this launch, so every enhancement stays off until \
             a launch without it. Changes made here are saved for later launches."
                .to_string(),
            LineStyle::Dim,
        ));
    }
    if let Some(notice) = &page.notice {
        lines.push((notice.clone(), LineStyle::Bad));
    }

    let mut rows = vec![PageRow {
        text: format!("Preset - your mix: {}", page.mix_text()),
        style: RowStyle::Heading,
        action: None,
        focused: false,
    }];
    let mix = page.mix();
    for (index, choice) in page.presets.iter().enumerate() {
        let chosen = mix == MixLabel::Preset(index);
        rows.push(PageRow {
            text: format!("({}) {}", if chosen { "*" } else { " " }, choice.label),
            style: RowStyle::Setting,
            action: Some(Action::SelectPreset(index)),
            focused: focused == Some(FocusItem::Preset(index)),
        });
    }
    let mut group: Option<&str> = None;
    for (index, row) in page.rows.iter().enumerate() {
        if row.role.is_cheat_group() {
            continue;
        }
        if group != Some(row.group.as_str()) {
            group = Some(row.group.as_str());
            rows.push(heading(&row.group));
        }
        rows.push(setting_row(page, index, focused));
    }
    if page.rows.iter().all(|row| row.role.is_cheat_group()) {
        rows.push(PageRow {
            text: "No enhancements are available in this build yet.".to_string(),
            style: RowStyle::Greyed,
            action: None,
            focused: false,
        });
    }
    if page.shows_cheats() {
        rows.push(PageRow {
            text: format!(
                "{} {CHEATS_GROUP}",
                if page.cheats_open { "v" } else { ">" }
            ),
            style: RowStyle::Heading,
            action: Some(Action::ToggleCheatsGroup),
            focused: focused == Some(FocusItem::CheatsGroup),
        });
        if page.cheats_open {
            for index in page.cheat_rows() {
                rows.push(setting_row(page, index, focused));
            }
        }
    }
    lines.push((
        "Changes apply when the game starts.".to_string(),
        LineStyle::Dim,
    ));
    let button = |label: &str, action: Action, item: FocusItem| {
        let text = if focused == Some(item) {
            format!("> {label} <")
        } else {
            label.to_string()
        };
        (text, action)
    };
    Page {
        title: "Enhancements".to_string(),
        lines,
        list: Vec::new(),
        rows,
        buttons: vec![
            button("Defaults", Action::SettingsDefaults, FocusItem::Defaults),
            button("Cancel", Action::Back, FocusItem::Cancel),
            button("Save", Action::SaveSettings, FocusItem::Save),
        ],
    }
}

fn heading(text: &str) -> PageRow {
    PageRow {
        text: text.to_string(),
        style: RowStyle::Heading,
        action: None,
        focused: false,
    }
}

fn setting_row(page: &SettingsPage, index: usize, focused: Option<FocusItem>) -> PageRow {
    let row = &page.rows[index];
    let value = match (&row.kind, &row.value) {
        (_, RowValue::Switch(on)) => format!("[{}]", if *on { "x" } else { " " }),
        (RowKind::Choice(choices), RowValue::Choice(value)) => {
            let label = choices
                .iter()
                .find(|(choice, _)| choice == value)
                .map_or(value.as_str(), |(_, label)| label.as_str());
            format!("[{label}]")
        }
        (RowKind::Switch, RowValue::Choice(value)) => format!("[{value}]"),
    };
    let mut text = format!("{value} {}", row.label);
    for tag in &row.tags {
        text.push_str(&format!(" ({tag})"));
    }
    PageRow {
        text,
        style: if page.row_greyed(index) {
            RowStyle::Greyed
        } else {
            RowStyle::Setting
        },
        action: Some(Action::ToggleSetting(index)),
        focused: focused == Some(FocusItem::Row(index)),
    }
}

/// The confirmation the page is waiting on. The wording is our own
/// (design 7.2.1 for cheats).
fn confirm_page(page: &SettingsPage, confirm: &Confirm) -> Page {
    let body = |text: &str| (text.to_string(), LineStyle::Body);
    let (title, lines, yes, no) = match confirm {
        Confirm::AllowCheats(_) => (
            "Allow cheats?".to_string(),
            vec![body(
                "Cheats change your party and the world in ways the original game never \
                 would. Saves you make afterwards are marked \"modified\". You can turn \
                 cheats off again at any time; that does not unmark a save.",
            )],
            "[Y] Allow cheats",
            "[N] Cancel",
        ),
        Confirm::Defaults => (
            "Back to the defaults?".to_string(),
            vec![body(
                "Every enhancement turns off and every setting on this page goes back to \
                 its default: the original game. Nothing is saved until you choose Save.",
            )],
            "[Y] Use the defaults",
            "[N] Cancel",
        ),
        Confirm::KeepChanges(preset) => {
            let label = page
                .presets
                .get(*preset)
                .map_or("", |choice| choice.label.as_str());
            (
                format!("Switch to {label}?"),
                vec![body(&format!(
                    "You changed some settings of your current mix ({}). Keep those \
                     changes on top of {label}, or use {label} as it is?",
                    page.mix_text()
                ))],
                "[Y] Keep my changes",
                "[N] Use the preset as it is",
            )
        }
    };
    Page {
        title,
        lines,
        list: Vec::new(),
        rows: Vec::new(),
        buttons: vec![
            (yes.to_string(), Action::ConfirmYes),
            (no.to_string(), Action::ConfirmNo),
        ],
    }
}

/// The plain-language validation result, one styled line per paragraph.
pub fn review_lines(report: &ValidationReport) -> Vec<(String, LineStyle)> {
    let style = if report.is_usable() {
        LineStyle::Good
    } else {
        LineStyle::Bad
    };
    report
        .summary()
        .lines()
        .skip(1) // "Folder: ..." is shown above.
        .filter(|text| !text.trim().is_empty())
        .map(|text| {
            let line_style = if text.starts_with("Warning") || text.starts_with("Note") {
                LineStyle::Body
            } else {
                style
            };
            (text.trim().to_string(), line_style)
        })
        .collect()
}

fn text_bundle(text: &str, size: f32, color: Color) -> impl Bundle {
    (
        Text::new(text),
        TextFont {
            font_size: size,
            ..default()
        },
        TextColor(color),
    )
}

/// The setting rows, top to bottom, wrapping into further columns when the
/// window is not tall enough. A heading and the rows under it form one
/// block, so a column never ends on a heading. Each clickable row is a
/// button.
fn spawn_rows(content: &mut ChildSpawnerCommands, rows: &[PageRow]) {
    let mut blocks: Vec<&[PageRow]> = Vec::new();
    let mut start = 0;
    for (index, row) in rows.iter().enumerate() {
        if row.style == RowStyle::Heading && index > start {
            blocks.push(&rows[start..index]);
            start = index;
        }
    }
    if start < rows.len() {
        blocks.push(&rows[start..]);
    }
    content
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            flex_wrap: FlexWrap::Wrap,
            align_content: AlignContent::FlexStart,
            flex_grow: 1.0,
            min_height: Val::Px(0.0),
            column_gap: Val::Px(28.0),
            row_gap: Val::Px(8.0),
            ..default()
        })
        .with_children(|columns| {
            for block in blocks {
                columns
                    .spawn(Node {
                        flex_direction: FlexDirection::Column,
                        flex_shrink: 0.0,
                        row_gap: Val::Px(2.0),
                        ..default()
                    })
                    .with_children(|column| {
                        for row in block {
                            spawn_row(column, row);
                        }
                    });
            }
        });
}

fn spawn_row(column: &mut ChildSpawnerCommands, row: &PageRow) {
    let background = if row.focused { EGA_BLUE } else { EGA_BLACK };
    let color = if row.focused {
        EGA_WHITE
    } else {
        row.style.color()
    };
    let indent = if row.style == RowStyle::Heading {
        0.0
    } else {
        14.0
    };
    let node = Node {
        padding: UiRect::new(
            Val::Px(4.0 + indent),
            Val::Px(4.0),
            Val::Px(1.0),
            Val::Px(1.0),
        ),
        ..default()
    };
    match &row.action {
        Some(action) => {
            column
                .spawn((
                    Button,
                    WizardButton(action.clone()),
                    WizardRowButton,
                    node,
                    BackgroundColor(background),
                ))
                .with_children(|button| {
                    button.spawn(text_bundle(&row.text, 15.0, color));
                });
        }
        None => {
            column
                .spawn((node, BackgroundColor(background)))
                .with_children(|line| {
                    line.spawn(text_bundle(&row.text, 15.0, color));
                });
        }
    }
}

fn spawn_page(commands: &mut Commands, page: Page) {
    commands
        .spawn((
            WizardRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(28.0)),
                row_gap: Val::Px(12.0),
                ..default()
            },
            BackgroundColor(EGA_BLACK),
        ))
        .with_children(|root| {
            root.spawn(text_bundle("ULTIMA V  -  SETUP", 16.0, EGA_CYAN));
            root.spawn(text_bundle(&page.title, 30.0, EGA_YELLOW));
            // Long pages (the help text, many candidates) use a smaller
            // size, and the content area clips rather than pushing the
            // buttons off the window.
            let text_chars: usize = page.lines.iter().map(|(text, _)| text.len()).sum();
            let body_size = if text_chars > 900 || page.list.len() > 4 || !page.rows.is_empty() {
                15.0
            } else {
                18.0
            };
            root.spawn(Node {
                flex_direction: FlexDirection::Column,
                // The setting rows take whatever height is left, so they
                // can flow into a second column.
                flex_grow: if page.rows.is_empty() { 0.0 } else { 1.0 },
                flex_shrink: 1.0,
                min_height: Val::Px(0.0),
                row_gap: Val::Px(10.0),
                overflow: Overflow::clip_y(),
                ..default()
            })
            .with_children(|content| {
                for (text, style) in &page.lines {
                    content.spawn(text_bundle(text, body_size, style.color()));
                }
                for (label, action) in &page.list {
                    content
                        .spawn((
                            Button,
                            WizardButton(action.clone()),
                            Node {
                                width: Val::Percent(100.0),
                                flex_shrink: 0.0,
                                padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)),
                                border: UiRect::all(Val::Px(2.0)),
                                ..default()
                            },
                            BorderColor(EGA_LIGHT_GREY),
                            BackgroundColor(EGA_BLUE),
                        ))
                        .with_children(|button| {
                            button.spawn(text_bundle(label, body_size.min(16.0), EGA_WHITE));
                        });
                }
                if !page.rows.is_empty() {
                    spawn_rows(content, &page.rows);
                }
            });
            root.spawn(Node {
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Wrap,
                flex_shrink: 0.0,
                column_gap: Val::Px(12.0),
                row_gap: Val::Px(8.0),
                margin: UiRect::top(Val::Px(12.0)),
                ..default()
            })
            .with_children(|row| {
                for (label, action) in &page.buttons {
                    row.spawn((
                        Button,
                        WizardButton(action.clone()),
                        Node {
                            padding: UiRect::axes(Val::Px(18.0), Val::Px(10.0)),
                            border: UiRect::all(Val::Px(2.0)),
                            ..default()
                        },
                        BorderColor(EGA_LIGHT_GREY),
                        BackgroundColor(EGA_BLUE),
                    ))
                    .with_children(|button| {
                        button.spawn(text_bundle(label, 20.0, EGA_WHITE));
                    });
                }
            });
        });
}

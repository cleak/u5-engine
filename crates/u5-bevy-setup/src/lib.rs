//! The windowed first-run setup wizard, as a Bevy plugin.
//!
//! Screens: welcome; the folders discovery found, each with its validation
//! status; "Choose folder..." through the host's native picker; a review of
//! the chosen folder that lists any missing files in plain language; "How to
//! get the files"; and a final confirmation that hands over to the game.
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

pub use model::{Action, CommitSummary, Screen, WizardHost, WizardModel, WizardOutcome};

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
        .insert_resource(WizardState {
            model: WizardModel::new(reason, candidates),
            dirty: true,
        })
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
    let action = if keys.just_pressed(KeyCode::Escape) {
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

/// What Enter does on each screen.
pub fn primary_action(screen: &Screen) -> Option<Action> {
    match screen {
        Screen::Welcome => Some(Action::Continue),
        Screen::Review { report, .. } if report.is_usable() => Some(Action::UseFolder),
        Screen::Done(_) => Some(Action::Start),
        Screen::Help => Some(Action::Back),
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
    (Changed<Interaction>, With<WizardButton>),
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
    pub buttons: Vec<(String, Action)>,
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
        Screen::Welcome => Page {
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
            buttons: vec![
                ("Continue".to_string(), Action::Continue),
                ("How to get the files".to_string(), Action::ShowHelp),
                ("Quit".to_string(), Action::Quit),
            ],
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
                buttons,
            }
        }
        Screen::Help => Page {
            title: "How to get the files".to_string(),
            lines: vec![line(&host.help_text(), LineStyle::Body)],
            list: Vec::new(),
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
            buttons: vec![
                ("Start the game".to_string(), Action::Start),
                ("Quit".to_string(), Action::Quit),
            ],
        },
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
            for (text, style) in &page.lines {
                root.spawn(text_bundle(text, 18.0, style.color()));
            }
            for (label, action) in &page.list {
                root.spawn((
                    Button,
                    WizardButton(action.clone()),
                    Node {
                        width: Val::Percent(100.0),
                        padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)),
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BorderColor(EGA_LIGHT_GREY),
                    BackgroundColor(EGA_BLUE),
                ))
                .with_children(|button| {
                    button.spawn(text_bundle(label, 16.0, EGA_WHITE));
                });
            }
            root.spawn(Node {
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Wrap,
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

//! Wizard flows: the state machine directly, and the Bevy plugin headless
//! (MinimalPlugins, buttons "pressed" by setting their Interaction).

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, channel};
use std::sync::{Arc, Mutex};

use bevy::prelude::*;
use u5_bevy_setup::*;
use u5_gamedata::{Candidate, Discovered, MANIFEST, ValidationReport, validate_dir};

struct Scratch(PathBuf);

impl Scratch {
    fn new(label: &str) -> Self {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "u5-bevy-setup-{label}-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn populate(dir: &Path, skip: &[&str]) {
    fs::create_dir_all(dir).unwrap();
    for file in MANIFEST {
        if !skip.contains(&file.name) {
            fs::write(dir.join(file.name), format!("synthetic {}", file.name)).unwrap();
        }
    }
}

#[derive(Default)]
struct FakeHost {
    picks: Mutex<Vec<Option<PathBuf>>>,
    no_picker: bool,
    fail_commit: bool,
    committed: Mutex<Vec<PathBuf>>,
}

impl WizardHost for FakeHost {
    fn discover(&self) -> Vec<Discovered> {
        Vec::new()
    }
    fn validate(&self, path: &Path) -> ValidationReport {
        validate_dir(path)
    }
    fn commit(&self, path: &Path) -> Result<CommitSummary, String> {
        if self.fail_commit {
            return Err("disk full".to_string());
        }
        self.committed.lock().unwrap().push(path.to_path_buf());
        Ok(CommitSummary {
            data_dir: path.join("managed"),
            message: format!("Using {}", path.display()),
        })
    }
    fn start_folder_pick(&self) -> Option<Receiver<Option<PathBuf>>> {
        if self.no_picker {
            return None;
        }
        let (sender, receiver) = channel();
        let answer = self.picks.lock().unwrap().pop().flatten();
        sender.send(answer).unwrap();
        Some(receiver)
    }
    fn help_text(&self) -> String {
        "Buy it on GOG.com; on Linux use innoextract.".to_string()
    }
}

fn discovered(dir: &Path) -> Discovered {
    Discovered {
        candidate: Candidate::new(dir, "test", 0),
        data_root: dir.to_path_buf(),
        report: validate_dir(dir),
    }
}

#[test]
fn candidate_flow_completes() {
    let scratch = Scratch::new("candidate");
    let game = scratch.0.join("game");
    populate(&game, &[]);
    let host = FakeHost::default();
    let mut model = WizardModel::new("No data yet.", vec![discovered(&game)]);
    assert_eq!(model.screen, Screen::Welcome);
    assert!(model.apply(Action::Continue, &host));
    assert_eq!(model.screen, Screen::Candidates);
    let page = page_for(&model, &host);
    assert_eq!(page.list.len(), 1);
    assert!(page.list[0].0.contains("ready"), "{}", page.list[0].0);

    assert!(model.apply(Action::SelectCandidate(0), &host));
    assert!(matches!(&model.screen, Screen::Review { report, .. } if report.is_usable()));
    assert_eq!(primary_action(&model.screen), Some(Action::UseFolder));
    assert!(model.apply(Action::UseFolder, &host));
    assert!(matches!(model.screen, Screen::Done(_)));
    assert!(model.apply(Action::Start, &host));
    match model.outcome.clone().unwrap() {
        WizardOutcome::Completed(summary) => assert_eq!(summary.data_dir, game.join("managed")),
        other => panic!("{other:?}"),
    }
    assert_eq!(*host.committed.lock().unwrap(), vec![game]);
    // Finished: further actions are ignored.
    assert!(!model.apply(Action::Back, &host));
}

#[test]
fn chosen_folder_with_missing_files_lists_them_and_cannot_be_used() {
    let scratch = Scratch::new("missing");
    let game = scratch.0.join("partial");
    populate(&game, &["TILES.16", "IBM.CH"]);
    let host = FakeHost {
        picks: Mutex::new(vec![Some(game.clone())]),
        ..FakeHost::default()
    };
    let mut model = WizardModel::new("", Vec::new());
    model.apply(Action::Continue, &host);
    assert!(model.apply(Action::ChooseFolder, &host));
    // The fake picker answers immediately, as macOS's does.
    let Screen::Review { report, .. } = &model.screen else {
        panic!("{:?}", model.screen);
    };
    assert!(!report.is_usable());
    let page = page_for(&model, &host);
    assert_eq!(page.title, "This folder cannot be used");
    let text: Vec<&str> = page.lines.iter().map(|(text, _)| text.as_str()).collect();
    assert!(
        text.iter().any(|line| line.contains("IBM.CH, TILES.16")),
        "{text:#?}"
    );
    assert!(
        text.iter().any(|line| line.contains("2 of the 63 files")),
        "{text:#?}"
    );
    assert!(page.lines.iter().any(|(_, style)| *style == LineStyle::Bad));
    assert!(
        !page
            .buttons
            .iter()
            .any(|(_, action)| *action == Action::UseFolder)
    );
    assert!(!model.apply(Action::UseFolder, &host));
    assert!(host.committed.lock().unwrap().is_empty());
}

#[test]
fn cancelled_picker_and_missing_picker_leave_a_notice() {
    let host = FakeHost {
        picks: Mutex::new(vec![None]),
        ..FakeHost::default()
    };
    let mut model = WizardModel::new("", Vec::new());
    model.apply(Action::Continue, &host);
    model.apply(Action::ChooseFolder, &host);
    assert_eq!(model.screen, Screen::Candidates);
    assert_eq!(model.notice.as_deref(), Some("No folder was chosen."));

    let no_picker = FakeHost {
        no_picker: true,
        ..FakeHost::default()
    };
    model.apply(Action::ChooseFolder, &no_picker);
    assert!(
        model
            .notice
            .as_deref()
            .unwrap()
            .contains("--setup <folder>")
    );
}

#[test]
fn pending_picker_waits_on_its_own_screen() {
    struct SlowHost(Mutex<Option<std::sync::mpsc::Sender<Option<PathBuf>>>>);
    impl WizardHost for SlowHost {
        fn discover(&self) -> Vec<Discovered> {
            Vec::new()
        }
        fn validate(&self, path: &Path) -> ValidationReport {
            validate_dir(path)
        }
        fn commit(&self, _: &Path) -> Result<CommitSummary, String> {
            Err("unused".into())
        }
        fn start_folder_pick(&self) -> Option<Receiver<Option<PathBuf>>> {
            let (sender, receiver) = channel();
            *self.0.lock().unwrap() = Some(sender);
            Some(receiver)
        }
        fn help_text(&self) -> String {
            String::new()
        }
    }
    let host = SlowHost(Mutex::new(None));
    let mut model = WizardModel::new("", Vec::new());
    model.apply(Action::Continue, &host);
    model.apply(Action::ChooseFolder, &host);
    assert_eq!(model.screen, Screen::Picking);
    assert!(!model.poll(&host));
    let scratch = Scratch::new("slow");
    host.0
        .lock()
        .unwrap()
        .take()
        .unwrap()
        .send(Some(scratch.0.clone()))
        .unwrap();
    assert!(model.poll(&host));
    assert!(matches!(model.screen, Screen::Review { .. }));
}

#[test]
fn help_returns_to_where_it_was_opened_and_commit_errors_show() {
    let scratch = Scratch::new("help");
    let game = scratch.0.join("game");
    populate(&game, &[]);
    let host = FakeHost {
        fail_commit: true,
        ..FakeHost::default()
    };
    let mut model = WizardModel::new("", vec![discovered(&game)]);
    model.apply(Action::Continue, &host);
    model.apply(Action::SelectCandidate(0), &host);
    let review = model.screen.clone();
    model.apply(Action::ShowHelp, &host);
    assert_eq!(model.screen, Screen::Help);
    assert!(page_for(&model, &host).lines[0].0.contains("innoextract"));
    model.apply(Action::Back, &host);
    assert_eq!(model.screen, review);
    model.apply(Action::UseFolder, &host);
    let Screen::Review { error, .. } = &model.screen else {
        panic!()
    };
    assert_eq!(error.as_deref(), Some("disk full"));
    assert!(
        page_for(&model, &host)
            .lines
            .iter()
            .any(|(text, _)| text.contains("Setup failed: disk full"))
    );
    model.apply(Action::Quit, &host);
    assert_eq!(model.outcome, Some(WizardOutcome::Cancelled));
}

fn press(app: &mut App, action: Action) {
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &WizardButton)>();
    let entity = query
        .iter(world)
        .find(|(_, button)| button.0 == action)
        .map(|(entity, _)| entity)
        .unwrap_or_else(|| panic!("no {action:?} button on screen"));
    world.entity_mut(entity).insert(Interaction::Pressed);
    app.update();
}

fn screen(app: &App) -> Screen {
    app.world().resource::<WizardState>().model.screen.clone()
}

#[test]
fn bevy_plugin_drives_the_flow_headlessly() {
    let scratch = Scratch::new("bevy");
    let game = scratch.0.join("game");
    populate(&game, &[]);
    let host: Arc<dyn WizardHost> = Arc::new(FakeHost::default());
    let result = WizardResult::default();
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .insert_resource(WizardHostResource(host))
        .insert_resource(WizardState {
            model: WizardModel::new("Nothing found.", vec![discovered(&game)]),
            dirty: true,
        })
        .insert_resource(result.clone())
        .add_plugins(SetupWizardPlugin);
    app.update();
    let roots = app
        .world_mut()
        .query_filtered::<Entity, With<WizardRoot>>()
        .iter(app.world())
        .count();
    assert_eq!(roots, 1);

    press(&mut app, Action::Continue);
    assert_eq!(screen(&app), Screen::Candidates);
    // The old page was replaced, not stacked.
    let roots = app
        .world_mut()
        .query_filtered::<Entity, With<WizardRoot>>()
        .iter(app.world())
        .count();
    assert_eq!(roots, 1);
    press(&mut app, Action::SelectCandidate(0));
    assert!(matches!(screen(&app), Screen::Review { .. }));
    press(&mut app, Action::UseFolder);
    assert!(matches!(screen(&app), Screen::Done(_)));
    press(&mut app, Action::Start);
    let outcome = result.0.lock().unwrap().clone();
    assert!(
        matches!(outcome, Some(WizardOutcome::Completed(_))),
        "{outcome:?}"
    );
    let exits = app.world().resource::<Events<AppExit>>();
    assert!(!exits.is_empty());
}

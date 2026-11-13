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
    // Quitting after the choice is saved is not a cancellation.
    let mut quitter = WizardModel::new("", vec![discovered(&game)]);
    quitter.apply(Action::Continue, &host);
    quitter.apply(Action::SelectCandidate(0), &host);
    quitter.apply(Action::UseFolder, &host);
    quitter.apply(Action::Quit, &host);
    assert!(matches!(quitter.outcome, Some(WizardOutcome::SavedOnly(_))));

    assert!(model.apply(Action::Start, &host));
    match model.outcome.clone().unwrap() {
        WizardOutcome::Completed(summary) => assert_eq!(summary.data_dir, game.join("managed")),
        other => panic!("{other:?}"),
    }
    assert_eq!(host.committed.lock().unwrap().len(), 2);
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

// ---- the Enhancements page (Phase B design 1.2.2, package M0-08) ----

/// A host with settings: two ordinary rows a preset governs, one
/// preference no preset touches, and the Cheats group.
struct SettingsHost {
    locked: bool,
    preset: usize,
    fail_save: bool,
    /// Adds an indented sub-option under the Display row.
    sub_row: bool,
    /// Gives the first row ("Sharp scaling") a help line.
    help: bool,
    saved: Mutex<Vec<SettingsChanges>>,
}

impl SettingsHost {
    fn new() -> Self {
        Self {
            locked: false,
            preset: 0,
            fail_save: false,
            sub_row: false,
            help: false,
            saved: Mutex::new(Vec::new()),
        }
    }
}

fn switch(key: &str, label: &str, group: &str, role: RowRole, on: bool) -> SettingRow {
    SettingRow {
        key: key.to_string(),
        label: label.to_string(),
        group: group.to_string(),
        tags: Vec::new(),
        kind: RowKind::Switch,
        role,
        value: RowValue::Switch(on),
        default: RowValue::Switch(false),
        indent: false,
        help: None,
    }
}

fn settings_fixture(locked: bool, preset: usize, sub_row: bool) -> SettingsRows {
    let recommended = preset == 1;
    let mut tier = switch(
        "journal.tier",
        "Spoilers",
        "Journal",
        RowRole::Setting,
        false,
    );
    tier.kind = RowKind::Choice(vec![
        ("1".to_string(), "1 Basic".to_string()),
        ("2".to_string(), "2 Goals".to_string()),
    ]);
    tier.value = RowValue::Choice("2".to_string());
    tier.default = RowValue::Choice("2".to_string());
    let mut sharp = switch(
        "display.sharp",
        "Sharp scaling",
        "Display",
        RowRole::Setting,
        recommended,
    );
    sharp.tags = vec!["wider view".to_string()];
    let mut crawl = switch(
        "display.crawl",
        "Messages over the map",
        "Display",
        RowRole::Setting,
        true,
    );
    crawl.indent = true;
    crawl.default = RowValue::Switch(true);
    let subs = if sub_row { vec![crawl] } else { Vec::new() };
    SettingsRows {
        rows: [
            vec![sharp],
            subs,
            vec![
                switch(
                    "combat.auto",
                    "Auto-combat",
                    "Combat",
                    RowRole::Setting,
                    recommended,
                ),
                tier,
                // Deliberately out of order: the page puts "Allow cheats"
                // first and "Watch game state" last.
                switch(
                    "cheats.state_watch",
                    "Watch game state",
                    "Cheats",
                    RowRole::WatchState,
                    false,
                ),
                switch(
                    "cheats.heal",
                    "Heal the party",
                    "Cheats",
                    RowRole::Cheat,
                    false,
                ),
                switch(
                    "cheats.enabled",
                    "Allow cheats",
                    "Cheats",
                    RowRole::AllowCheats,
                    false,
                ),
            ],
        ]
        .concat(),
        presets: vec![
            PresetChoice {
                key: "classic".to_string(),
                label: "Classic".to_string(),
                values: vec![
                    ("display.sharp".to_string(), RowValue::Switch(false)),
                    ("combat.auto".to_string(), RowValue::Switch(false)),
                ],
            },
            PresetChoice {
                key: "recommended".to_string(),
                label: "Recommended".to_string(),
                values: vec![
                    ("display.sharp".to_string(), RowValue::Switch(false)),
                    ("combat.auto".to_string(), RowValue::Switch(true)),
                ],
            },
            PresetChoice {
                key: "everything".to_string(),
                label: "Everything".to_string(),
                values: vec![
                    ("display.sharp".to_string(), RowValue::Switch(true)),
                    ("combat.auto".to_string(), RowValue::Switch(true)),
                    // A key this page has no row for is skipped.
                    ("maps.hidden".to_string(), RowValue::Switch(true)),
                ],
            },
        ],
        preset,
        locked,
        notes: Vec::new(),
    }
}

impl WizardHost for SettingsHost {
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
        None
    }
    fn help_text(&self) -> String {
        String::new()
    }
    fn settings_rows(&self) -> Option<SettingsRows> {
        let mut rows = settings_fixture(self.locked, self.preset, self.sub_row);
        if self.preset == 1 {
            // Recommended as stored has display.sharp off.
            rows.rows[0].value = RowValue::Switch(false);
        }
        if self.help {
            rows.rows[0].help = Some("Keeps every pixel square.".to_string());
        }
        Some(rows)
    }
    fn save_settings(&self, changes: &SettingsChanges) -> Result<(), String> {
        if self.fail_save {
            return Err("read-only folder".to_string());
        }
        self.saved.lock().unwrap().push(changes.clone());
        Ok(())
    }
}

fn page(model: &WizardModel) -> &SettingsPage {
    model
        .settings
        .as_ref()
        .expect("the Enhancements page is open")
}

fn row_of(model: &WizardModel, key: &str) -> usize {
    page(model)
        .rows
        .iter()
        .position(|row| row.key == key)
        .unwrap()
}

fn value_of(model: &WizardModel, key: &str) -> RowValue {
    page(model).rows[row_of(model, key)].value.clone()
}

fn row_texts(model: &WizardModel, host: &dyn WizardHost) -> Vec<String> {
    page_for(model, host)
        .rows
        .into_iter()
        .map(|row| row.text)
        .collect()
}

#[test]
fn hosts_without_settings_offer_no_page() {
    let host = FakeHost::default();
    let mut model = WizardModel::new("", Vec::new()).offer_settings(&host);
    assert!(!model.offers_settings);
    assert!(
        !page_for(&model, &host)
            .buttons
            .iter()
            .any(|(_, action)| *action == Action::OpenSettings)
    );
    assert!(!model.apply(Action::OpenSettings, &host));
    assert_eq!(model.screen, Screen::Welcome);
    // The settings-only wizard has nothing to show and finishes at once.
    let only = WizardModel::settings_only(&host);
    assert_eq!(
        only.outcome,
        Some(WizardOutcome::SettingsClosed { saved: false })
    );
    // The default save refuses.
    assert!(host.save_settings(&SettingsChanges::default()).is_err());
}

#[test]
fn welcome_opens_the_page_and_save_returns_there() {
    let host = SettingsHost::new();
    let mut model = WizardModel::new("", Vec::new()).offer_settings(&host);
    assert!(model.offers_settings);
    assert!(
        page_for(&model, &host)
            .buttons
            .iter()
            .any(|(label, action)| *action == Action::OpenSettings && label == "Enhancements...")
    );
    assert!(model.apply(Action::OpenSettings, &host));
    assert_eq!(model.screen, Screen::Enhancements);
    assert_eq!(primary_action(&model.screen), Some(Action::SaveSettings));
    let index = row_of(&model, "combat.auto");
    assert!(model.apply(Action::ToggleSetting(index), &host));
    assert!(model.apply(Action::SaveSettings, &host));
    assert_eq!(model.screen, Screen::Welcome);
    assert!(model.settings.is_none());
    assert_eq!(model.notice.as_deref(), Some("Settings saved."));
    assert!(model.outcome.is_none());
    // Cancel discards and goes back too.
    model.apply(Action::OpenSettings, &host);
    let index = row_of(&model, "combat.auto");
    model.apply(Action::ToggleSetting(index), &host);
    assert!(model.apply(Action::Back, &host));
    assert_eq!(model.screen, Screen::Welcome);
    assert_eq!(host.saved.lock().unwrap().len(), 1);
}

#[test]
fn presets_set_their_rows_and_changes_show_custom() {
    let host = SettingsHost::new();
    let mut model = WizardModel::settings_only(&host);
    assert_eq!(page(&model).mix(), MixLabel::Preset(0));
    let texts = row_texts(&model, &host);
    assert_eq!(texts[0], "Preset - your mix: Classic");
    assert_eq!(
        &texts[1..4],
        ["(*) Classic", "( ) Recommended", "( ) Everything"]
    );

    assert!(model.apply(Action::SelectPreset(2), &host));
    assert_eq!(value_of(&model, "display.sharp"), RowValue::Switch(true));
    assert_eq!(value_of(&model, "combat.auto"), RowValue::Switch(true));
    // A preference no preset names is left alone.
    assert_eq!(
        value_of(&model, "journal.tier"),
        RowValue::Choice("2".into())
    );
    assert_eq!(page(&model).mix_text(), "Everything");

    let sharp = row_of(&model, "display.sharp");
    model.apply(Action::ToggleSetting(sharp), &host);
    // Everything without sharp scaling is exactly Recommended: the base
    // is tried first, then the presets in order.
    assert_eq!(page(&model).mix(), MixLabel::Preset(1));
    let auto = row_of(&model, "combat.auto");
    model.apply(Action::ToggleSetting(auto), &host);
    model.apply(Action::ToggleSetting(sharp), &host);
    assert_eq!(page(&model).mix(), MixLabel::Custom(2));
    assert_eq!(
        row_texts(&model, &host)[0],
        "Preset - your mix: Custom (based on Everything)"
    );
    assert!(
        row_texts(&model, &host)[1..4]
            .iter()
            .all(|text| text.starts_with("( )"))
    );

    // Choosing a preset over changes asks whether to keep them.
    assert!(model.apply(Action::SelectPreset(1), &host));
    assert_eq!(page(&model).confirm, Some(Confirm::KeepChanges(1)));
    let question = page_for(&model, &host);
    assert_eq!(question.title, "Switch to Recommended?");
    // Nothing else happens while the question is open.
    assert!(!model.apply(Action::ToggleSetting(sharp), &host));
    assert!(model.apply(Action::ConfirmYes, &host));
    // Kept: combat.auto stays off on top of Recommended.
    assert_eq!(value_of(&model, "combat.auto"), RowValue::Switch(false));
    assert_eq!(page(&model).mix(), MixLabel::Preset(0));
    assert_eq!(page(&model).base, 1);

    // Recommended with combat.auto off is a change on top of it, so
    // Everything asks too.
    model.apply(Action::SelectPreset(2), &host);
    model.apply(Action::ConfirmNo, &host);
    assert_eq!(page(&model).mix(), MixLabel::Preset(2));
    model.apply(Action::ToggleSetting(auto), &host);
    model.apply(Action::SelectPreset(1), &host);
    assert!(model.apply(Action::ConfirmNo, &host));
    // Taken as it is.
    assert_eq!(value_of(&model, "combat.auto"), RowValue::Switch(true));
    assert_eq!(value_of(&model, "display.sharp"), RowValue::Switch(false));
    assert_eq!(page(&model).mix(), MixLabel::Preset(1));

    // Back cancels the question and the choice.
    model.apply(Action::ToggleSetting(auto), &host);
    model.apply(Action::SelectPreset(2), &host);
    assert!(model.apply(Action::Back, &host));
    assert_eq!(page(&model).confirm, None);
    assert_eq!(model.screen, Screen::Enhancements);
    assert_eq!(page(&model).base, 1);
}

/// A sub-option (the grow-left rows, design 2.4.1) is drawn indented under
/// the row before it, and toggles like any row.
#[test]
fn a_sub_option_is_indented_under_its_parent() {
    let mut host = SettingsHost::new();
    host.sub_row = true;
    let mut model = WizardModel::settings_only(&host);
    let texts = row_texts(&model, &host);
    let sharp_at = texts
        .iter()
        .position(|text| text == "[ ] Sharp scaling (wider view)")
        .unwrap();
    assert_eq!(texts[sharp_at + 1], "    [x] Messages over the map");
    assert_eq!(
        texts.iter().filter(|text| text.starts_with("    ")).count(),
        1
    );
    let crawl = row_of(&model, "display.crawl");
    assert!(model.apply(Action::ToggleSetting(crawl), &host));
    assert_eq!(value_of(&model, "display.crawl"), RowValue::Switch(false));
    assert!(row_texts(&model, &host).contains(&"    [ ] Messages over the map".to_string()));
}

#[test]
fn cheats_group_is_collapsed_and_asks_before_allowing() {
    let host = SettingsHost::new();
    let mut model = WizardModel::settings_only(&host);
    let texts = row_texts(&model, &host);
    assert_eq!(texts.last().unwrap(), "> Cheats (changes your game)");
    assert!(!texts.iter().any(|text| text.contains("Allow cheats")));
    // Ordinary groups get headings, in the host's order; tags follow
    // labels.
    let headings: Vec<String> = page_for(&model, &host)
        .rows
        .into_iter()
        .filter(|row| row.style == RowStyle::Heading)
        .map(|row| row.text)
        .collect();
    assert_eq!(
        headings[1..],
        [
            "Display",
            "Combat",
            "Journal",
            "> Cheats (changes your game)"
        ]
    );
    assert!(texts.contains(&"[ ] Sharp scaling (wider view)".to_string()));
    assert!(texts.contains(&"[2 Goals] Spoilers".to_string()));
    // Hidden rows cannot be toggled.
    let heal = row_of(&model, "cheats.heal");
    assert!(!model.apply(Action::ToggleSetting(heal), &host));

    assert!(model.apply(Action::ToggleCheatsGroup, &host));
    let rows = page_for(&model, &host).rows;
    let open = rows
        .iter()
        .position(|row| row.text == "v Cheats (changes your game)")
        .unwrap();
    let group: Vec<(&str, RowStyle)> = rows[open + 1..]
        .iter()
        .map(|row| (row.text.as_str(), row.style))
        .collect();
    assert_eq!(
        group,
        [
            ("[ ] Allow cheats", RowStyle::Setting),
            ("[ ] Heal the party", RowStyle::Greyed),
            ("[ ] Watch game state", RowStyle::Setting),
        ]
    );
    // Greyed until allowed; Watch game state never is.
    assert!(!model.apply(Action::ToggleSetting(heal), &host));
    let watch = row_of(&model, "cheats.state_watch");
    assert!(model.apply(Action::ToggleSetting(watch), &host));
    assert_eq!(
        value_of(&model, "cheats.state_watch"),
        RowValue::Switch(true)
    );

    // Allow cheats asks first; No leaves it off.
    let allow = row_of(&model, "cheats.enabled");
    assert!(model.apply(Action::ToggleSetting(allow), &host));
    assert_eq!(page(&model).confirm, Some(Confirm::AllowCheats(allow)));
    let question = page_for(&model, &host);
    assert_eq!(question.title, "Allow cheats?");
    assert!(question.lines[0].0.contains("marked \"modified\""));
    assert_eq!(
        question.buttons,
        [
            ("[Y] Allow cheats".to_string(), Action::ConfirmYes),
            ("[N] Cancel".to_string(), Action::ConfirmNo),
        ]
    );
    model.apply(Action::ConfirmNo, &host);
    assert_eq!(value_of(&model, "cheats.enabled"), RowValue::Switch(false));
    assert!(page(&model).row_greyed(heal));

    model.apply(Action::ToggleSetting(allow), &host);
    model.apply(Action::ConfirmYes, &host);
    assert_eq!(value_of(&model, "cheats.enabled"), RowValue::Switch(true));
    assert!(!page(&model).row_greyed(heal));
    assert!(model.apply(Action::ToggleSetting(heal), &host));
    assert_eq!(value_of(&model, "cheats.heal"), RowValue::Switch(true));
    // Turning cheats off needs no question, and greys the cheat again.
    model.apply(Action::ToggleSetting(allow), &host);
    assert_eq!(page(&model).confirm, None);
    assert!(page(&model).row_greyed(heal));
    assert!(!page(&model).row_greyed(watch));
}

#[test]
fn cheats_group_is_hidden_under_the_lock() {
    let host = SettingsHost {
        locked: true,
        ..SettingsHost::new()
    };
    let mut model = WizardModel::settings_only(&host);
    let shown = page_for(&model, &host);
    assert!(!shown.rows.iter().any(|row| row.text.contains("Cheats")));
    assert!(
        !shown
            .rows
            .iter()
            .any(|row| row.text.contains("Watch game state"))
    );
    assert!(
        shown
            .lines
            .iter()
            .any(|(text, _)| text.contains("parity lock"))
    );
    assert!(!page(&model).focus_items().contains(&FocusItem::CheatsGroup));
    assert!(!model.apply(Action::ToggleCheatsGroup, &host));
    let watch = row_of(&model, "cheats.state_watch");
    assert!(!model.apply(Action::ToggleSetting(watch), &host));
    // Ordinary rows still save for later launches.
    let auto = row_of(&model, "combat.auto");
    assert!(model.apply(Action::ToggleSetting(auto), &host));
}

#[test]
fn save_stores_only_changed_keys() {
    let host = SettingsHost::new();
    // Nothing changed: no host call, and the page closes unsaved.
    let mut model = WizardModel::settings_only(&host);
    let auto = row_of(&model, "combat.auto");
    model.apply(Action::ToggleSetting(auto), &host);
    model.apply(Action::ToggleSetting(auto), &host);
    assert!(model.apply(Action::SaveSettings, &host));
    assert_eq!(
        model.outcome,
        Some(WizardOutcome::SettingsClosed { saved: false })
    );
    assert!(host.saved.lock().unwrap().is_empty());

    // One row changed: that row alone.
    let mut model = WizardModel::settings_only(&host);
    let tier = row_of(&model, "journal.tier");
    model.apply(Action::ToggleSetting(tier), &host);
    assert_eq!(
        value_of(&model, "journal.tier"),
        RowValue::Choice("1".into())
    );
    model.apply(Action::SaveSettings, &host);
    assert_eq!(
        model.outcome,
        Some(WizardOutcome::SettingsClosed { saved: true })
    );
    assert_eq!(
        host.saved.lock().unwrap().pop().unwrap(),
        SettingsChanges {
            preset: None,
            values: vec![("journal.tier".to_string(), RowValue::Choice("1".into()))],
        }
    );

    // A new preset: the preset, the rows it changed, and a row the player
    // set apart from it although it equals what was stored.
    let stored = SettingsHost {
        preset: 1,
        ..SettingsHost::new()
    };
    let mut model = WizardModel::settings_only(&stored);
    assert_eq!(page(&model).mix(), MixLabel::Preset(1));
    model.apply(Action::SelectPreset(2), &stored);
    let auto = row_of(&model, "combat.auto");
    model.apply(Action::ToggleSetting(auto), &stored);
    model.apply(Action::SaveSettings, &stored);
    assert_eq!(
        stored.saved.lock().unwrap().pop().unwrap(),
        SettingsChanges {
            preset: Some("everything".to_string()),
            values: vec![
                ("display.sharp".to_string(), RowValue::Switch(true)),
                ("combat.auto".to_string(), RowValue::Switch(false)),
            ],
        }
    );

    // Defaults asks, then puts every row back, and selects Classic.
    let mut model = WizardModel::settings_only(&stored);
    model.apply(Action::SettingsDefaults, &stored);
    assert_eq!(page(&model).confirm, Some(Confirm::Defaults));
    model.apply(Action::ConfirmYes, &stored);
    assert_eq!(page(&model).mix(), MixLabel::Preset(0));
    model.apply(Action::SaveSettings, &stored);
    assert_eq!(
        stored.saved.lock().unwrap().pop().unwrap(),
        SettingsChanges {
            preset: Some("classic".to_string()),
            values: vec![("combat.auto".to_string(), RowValue::Switch(false))],
        }
    );

    // A failed save keeps the page open with the reason.
    let failing = SettingsHost {
        fail_save: true,
        ..SettingsHost::new()
    };
    let mut model = WizardModel::settings_only(&failing);
    model.apply(Action::ToggleSetting(auto), &failing);
    model.apply(Action::SaveSettings, &failing);
    assert_eq!(model.screen, Screen::Enhancements);
    assert!(model.outcome.is_none());
    assert!(
        page_for(&model, &failing)
            .lines
            .iter()
            .any(|(text, _)| text == "Settings were not saved: read-only folder")
    );
    // Cancel, then: the game still starts.
    model.apply(Action::Back, &failing);
    assert_eq!(
        model.outcome,
        Some(WizardOutcome::SettingsClosed { saved: false })
    );
    // Closing the window is not Cancel: nothing starts.
    let mut closed = WizardModel::settings_only(&failing);
    closed.apply(Action::Quit, &failing);
    assert_eq!(closed.outcome, Some(WizardOutcome::Cancelled));
}

/// A row's help line (design 10.2.1, build plan M3-17) shows while the row
/// has the focus. The line stays, blank, while another item has it, so the
/// rows never move; a page with no help lines has no such line.
#[test]
fn the_focused_rows_help_line_shows_under_the_notes() {
    let help_lines = |model: &WizardModel, host: &dyn WizardHost| {
        page_for(model, host)
            .lines
            .into_iter()
            .skip_while(|(text, _)| text != "Changes apply when the game starts.")
            .skip(1)
            .collect::<Vec<_>>()
    };
    let plain = SettingsHost::new();
    let model = WizardModel::settings_only(&plain);
    assert!(help_lines(&model, &plain).is_empty());

    let host = SettingsHost {
        help: true,
        ..SettingsHost::new()
    };
    let mut model = WizardModel::settings_only(&host);
    // The focus starts on a preset: the line is there, blank.
    assert_eq!(
        help_lines(&model, &host),
        [(HELP_PLACEHOLDER.to_string(), LineStyle::Body)]
    );
    let sharp = row_of(&model, "display.sharp");
    while page(&model).focused() != Some(FocusItem::Row(sharp)) {
        model.apply(Action::FocusNext, &host);
    }
    assert_eq!(
        help_lines(&model, &host),
        [("Keeps every pixel square.".to_string(), LineStyle::Body)]
    );
    model.apply(Action::FocusNext, &host);
    assert_eq!(
        help_lines(&model, &host),
        [(HELP_PLACEHOLDER.to_string(), LineStyle::Body)]
    );
}

#[test]
fn focus_walks_presets_rows_group_and_buttons() {
    let host = SettingsHost::new();
    let mut model = WizardModel::settings_only(&host);
    let items = page(&model).focus_items();
    assert_eq!(
        items,
        [
            FocusItem::Preset(0),
            FocusItem::Preset(1),
            FocusItem::Preset(2),
            FocusItem::Row(0),
            FocusItem::Row(1),
            FocusItem::Row(2),
            FocusItem::CheatsGroup,
            FocusItem::Defaults,
            FocusItem::Cancel,
            FocusItem::Save,
        ]
    );
    assert!(model.apply(Action::FocusPrev, &host));
    assert_eq!(page(&model).focused(), Some(FocusItem::Save));
    assert!(model.apply(Action::FocusNext, &host));
    assert_eq!(page(&model).focused(), Some(FocusItem::Preset(0)));
    for _ in 0..6 {
        model.apply(Action::FocusNext, &host);
    }
    assert_eq!(page(&model).focused(), Some(FocusItem::CheatsGroup));
    assert!(model.apply(Action::ActivateFocus, &host));
    assert!(page(&model).cheats_open);
    model.apply(Action::FocusNext, &host);
    assert_eq!(page(&model).focused(), Some(FocusItem::Row(5)));
    // The focused row is marked on the page.
    let focused: Vec<String> = page_for(&model, &host)
        .rows
        .into_iter()
        .filter(|row| row.focused)
        .map(|row| row.text)
        .collect();
    assert_eq!(focused, ["[ ] Allow cheats"]);
}

fn key_press(app: &mut App, key: KeyCode) {
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.press(key);
    }
    app.update();
    let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    keys.release(key);
    keys.clear();
}

#[test]
fn core_plugin_drives_the_page_from_the_keyboard() {
    let host = Arc::new(SettingsHost::new());
    let dyn_host: Arc<dyn WizardHost> = host.clone();
    let result = WizardResult::default();
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<ButtonInput<KeyCode>>()
        .insert_resource(WizardHostResource(dyn_host.clone()))
        .insert_resource(WizardState {
            model: WizardModel::settings_only(dyn_host.as_ref()),
            dirty: true,
        })
        .insert_resource(result.clone())
        .add_plugins(WizardCorePlugin);
    app.update();
    // Down to the Recommended preset, choose it with Space.
    key_press(&mut app, KeyCode::ArrowDown);
    key_press(&mut app, KeyCode::Space);
    let state = app.world().resource::<WizardState>();
    assert!(state.dirty);
    assert_eq!(page(&state.model).mix(), MixLabel::Preset(1));
    // Tab to the Cheats group, open it, step to Allow cheats, ask, answer Y.
    for _ in 0..5 {
        key_press(&mut app, KeyCode::Tab);
    }
    key_press(&mut app, KeyCode::Space);
    key_press(&mut app, KeyCode::ArrowDown);
    key_press(&mut app, KeyCode::Space);
    assert!(matches!(
        page(&app.world().resource::<WizardState>().model).confirm,
        Some(Confirm::AllowCheats(_))
    ));
    key_press(&mut app, KeyCode::KeyY);
    // Enter saves and finishes the settings-only wizard.
    key_press(&mut app, KeyCode::Enter);
    assert_eq!(
        result.0.lock().unwrap().clone(),
        Some(WizardOutcome::SettingsClosed { saved: true })
    );
    assert!(!app.world().resource::<Events<AppExit>>().is_empty());
    assert_eq!(
        host.saved.lock().unwrap().clone(),
        [SettingsChanges {
            preset: Some("recommended".to_string()),
            values: vec![
                ("combat.auto".to_string(), RowValue::Switch(true)),
                ("cheats.enabled".to_string(), RowValue::Switch(true)),
            ],
        }]
    );
}

#[test]
fn bevy_plugin_renders_the_page_and_takes_clicks() {
    let host: Arc<dyn WizardHost> = Arc::new(SettingsHost::new());
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .insert_resource(WizardHostResource(host.clone()))
        .insert_resource(WizardState {
            model: WizardModel::new("", Vec::new()).offer_settings(host.as_ref()),
            dirty: true,
        })
        .insert_resource(WizardResult::default())
        .add_plugins(SetupWizardPlugin);
    app.update();
    press(&mut app, Action::OpenSettings);
    assert_eq!(screen(&app), Screen::Enhancements);
    let rows = app
        .world_mut()
        .query_filtered::<Entity, With<WizardRowButton>>()
        .iter(app.world())
        .count();
    // Three presets, three rows and the Cheats group heading.
    assert_eq!(rows, 7);
    press(&mut app, Action::ToggleSetting(1));
    let model = &app.world().resource::<WizardState>().model;
    assert_eq!(page(model).rows[1].value, RowValue::Switch(true));
    press(&mut app, Action::Back);
    assert_eq!(screen(&app), Screen::Welcome);
}

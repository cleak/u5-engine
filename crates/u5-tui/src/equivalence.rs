//! The trajectory-equivalence harness, terminal layer (Phase B design
//! 1.3.12, layer T).
//!
//! An enhancement is equivalent when it does not change the game:
//!
//! - a class (a) feature, with its own events removed from the trace, must
//!   pass through exactly the states of the all-off run;
//! - a class (b) feature must pass through the states of an all-off run in
//!   which the commands it typed are typed as ordinary commands at the same
//!   points.
//!
//! A feature's events in a play script are its `@` tokens
//! ([`crate::play_loop::is_enhancement_token`]). With the feature on,
//! [`FeatureHooks`] intercepts them before the play-script loop parses
//! them, and records what the feature did. The off run then replays the
//! same script through [`ReplayHooks`]: each token the on run intercepted is
//! removed, or replaced by the commands a class (b) feature typed for it.
//! A token no feature took runs as 1.0.1 runs it, in both runs.
//!
//! Both runs are compared on the [`TrajectoryDigest`] after every command,
//! the final frame hash, the final state line, and the bytes of
//! [`SAVE_FILES`] after a scripted `Q`, `Y` into a scratch copy.
//!
//! [`run_equivalence_suite`] (`u5-engine --equivalence-suite OUT [--matrix
//! FILE] GAME_DIR`) runs every route-smoke case so, under Classic and under
//! each row of the matrix. Classic is also written out as a route-smoke
//! manifest, which must equal the 1.0.1 route-smoke manifest byte for byte.
//! [`check_equivalence`] runs the same comparison on any state and script,
//! which is how `cargo test` covers the harness without game files.

use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Write as _};
use std::path::Path;

pub use u5_runtime::enhancements::MatrixRow;
use u5_runtime::enhancements::{
    CliSetting, EffectClass, LockState, Preset, ResolvedEnhancements, Toggle, TrajectoryDigest,
    fnv1a64, trajectory_digest,
};
use u5_runtime::{
    BRIT_OOL_FILENAME, PlayState, SAVED_GAM_FILENAME, SAVED_OOL_FILENAME,
    TOWN_NPC_MUTATIONS_FILENAME, TileGraphicsDepth, UNDER_OOL_FILENAME, WORLD_PROGRESS_STATE_FILE,
    handle_play_key_input, load_tile_atlas,
};

use crate::enhancement_settings::{check_cli_key, resolve_launch};
use crate::gem_equivalence::{
    check_gem_expectation, compare_gem_runs, gem_consumption_cases, run_gem_case,
};
use crate::gem_map::{GemPng, TerminalGemMap};
use crate::play_loop::{
    NoScriptTokenHooks, ScriptTokenHooks, ScriptTokenIntercept, play_script_state_line,
    replay_play_script_commands_hooked,
};
use crate::route_smoke::{
    RouteSmokeReport, RouteTraceObserver, route_smoke_cases, route_smoke_temp_dir,
    run_route_smoke_case_traced, seed_route_smoke_save_files, write_route_smoke_manifest,
};

/// The files compared after the scripted save: the save itself, the
/// object files and the engine's two side files.
pub const SAVE_FILES: [&str; 6] = [
    SAVED_GAM_FILENAME,
    SAVED_OOL_FILENAME,
    BRIT_OOL_FILENAME,
    UNDER_OOL_FILENAME,
    WORLD_PROGRESS_STATE_FILE,
    TOWN_NPC_MUTATIONS_FILENAME,
];

/// The name of the configuration every suite run includes.
pub const CLASSIC_RUN: &str = "classic";

/// The report the suite writes into its output folder.
pub const REPORT_FILE: &str = "equivalence-report.tsv";

/// Classic's route-smoke manifest, written beside the report. It must be
/// byte-for-byte the manifest `--route-smoke --route-smoke-manifest` writes.
pub const CLASSIC_MANIFEST_FILE: &str = "classic-route-smoke-manifest.tsv";

/// An enhancement's terminal half: the `@` tokens it owns (design 1.3.12).
///
/// It sees the state read-only. A class (b) feature acts only through the
/// commands it returns, which run as if the player had typed them; a class
/// (a) feature returns none.
pub trait TerminalFeature {
    /// The catalogue toggle this feature belongs to.
    fn toggle(&self) -> Toggle;
    /// The feature's effect class, as the harness judges it.
    fn class(&self) -> EffectClass;
    /// Whether `token` (an `@` token, trimmed) is this feature's.
    fn owns_token(&self, token: &str) -> bool;
    /// Handles one of its tokens and returns the commands it types.
    fn on_token(&mut self, token: &str, state: &PlayState) -> Vec<String>;
    /// Before each command runs: an observer takes its probe here.
    fn before_command(&mut self, _state: &PlayState) {}
    /// After each command ran: an observer folds what it saw here.
    fn after_command(&mut self, _state: &PlayState) {}
}

/// The terminal features a resolved configuration turns on.
///
/// A feature chapter adds its line here, behind its own toggle, so with
/// every toggle off (or the parity lock set) this list is empty and the
/// play script is the 1.0.1 script.
///
/// - The gem map (chapter 10, class (a)): its record, kept in memory, and
///   `@gemmap`, a count-only summary of it.
pub fn terminal_features(resolved: &ResolvedEnhancements) -> Vec<Box<dyn TerminalFeature>> {
    let mut features: Vec<Box<dyn TerminalFeature>> = Vec::new();
    if resolved.lock != LockState::Off {
        return features;
    }
    if resolved.toggles.on(Toggle::GemMapEnhanced) {
        features.push(Box::new(TerminalGemMap::in_memory()));
    }
    features
}

/// One `@` token a feature took during the on run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TokenEvent {
    pub index: usize,
    pub token: String,
    pub class: EffectClass,
    pub emitted: Vec<String>,
}

/// The on run's hooks: the features that are on take their tokens, and
/// every token taken is recorded.
pub struct FeatureHooks {
    features: Vec<Box<dyn TerminalFeature>>,
    events: Vec<TokenEvent>,
}

impl FeatureHooks {
    pub fn new(features: Vec<Box<dyn TerminalFeature>>) -> Self {
        Self {
            features,
            events: Vec::new(),
        }
    }

    pub fn events(&self) -> &[TokenEvent] {
        &self.events
    }

    pub fn into_events(self) -> Vec<TokenEvent> {
        self.events
    }
}

impl ScriptTokenHooks for FeatureHooks {
    fn before_command(&mut self, state: &PlayState) {
        for feature in &mut self.features {
            feature.before_command(state);
        }
    }

    fn after_command(&mut self, state: &PlayState) {
        for feature in &mut self.features {
            feature.after_command(state);
        }
    }

    fn intercept(
        &mut self,
        index: usize,
        token: &str,
        state: &mut PlayState,
    ) -> ScriptTokenIntercept {
        let token = token.trim();
        let Some(feature) = self.features.iter_mut().find(|f| f.owns_token(token)) else {
            return ScriptTokenIntercept::PassThrough;
        };
        let emitted = feature.on_token(token, state);
        self.events.push(TokenEvent {
            index,
            token: token.to_string(),
            class: feature.class(),
            emitted: emitted.clone(),
        });
        if emitted.is_empty() {
            ScriptTokenIntercept::Consumed
        } else {
            ScriptTokenIntercept::Emit(emitted)
        }
    }
}

/// The off run's hooks: the on run's events, replayed with every feature
/// off. A class (b) token becomes the commands its feature typed; any other
/// token the on run took is removed. Tokens the on run did not take pass
/// through, as they did there.
#[derive(Clone, Debug, Default)]
pub struct ReplayHooks {
    events: BTreeMap<usize, Option<Vec<String>>>,
}

impl ReplayHooks {
    pub fn new(events: &[TokenEvent]) -> Self {
        let events = events
            .iter()
            .map(|event| {
                let typed = (event.class == EffectClass::InputDriven && !event.emitted.is_empty())
                    .then(|| event.emitted.clone());
                (event.index, typed)
            })
            .collect();
        Self { events }
    }
}

impl ScriptTokenHooks for ReplayHooks {
    fn intercept(
        &mut self,
        index: usize,
        _token: &str,
        _state: &mut PlayState,
    ) -> ScriptTokenIntercept {
        match self.events.get(&index) {
            None => ScriptTokenIntercept::PassThrough,
            Some(None) => ScriptTokenIntercept::Consumed,
            Some(Some(commands)) => ScriptTokenIntercept::Emit(commands.clone()),
        }
    }
}

/// One command of a run and the state after it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraceStep {
    /// The script command it came from.
    pub index: usize,
    /// What ran: the command, or a command a feature typed for a token.
    pub command: String,
    pub digest: TrajectoryDigest,
}

/// Everything one run is compared on.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RunRecord {
    pub steps: Vec<TraceStep>,
    pub final_digest: Option<TrajectoryDigest>,
    pub final_state_line: String,
    /// The final frame's hash; `None` where the run has no renderer.
    pub final_frame_hash: Option<u64>,
    /// FNV-1a-64 of each of [`SAVE_FILES`] after the scripted save, or
    /// `None` for a file the save did not write.
    pub saves: Vec<(&'static str, Option<u64>)>,
    /// Why the scripted save wrote nothing, if it failed. Some seeded
    /// route-smoke states cannot be saved at all; both runs must then fail
    /// the same way.
    pub save_error: Option<String>,
    /// The run's error, if it failed.
    pub error: Option<String>,
}

/// Where two runs part.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mismatch {
    pub what: String,
    pub off: String,
    pub on: String,
}

impl std::fmt::Display for Mismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: off {} / on {}", self.what, self.off, self.on)
    }
}

/// Compares an on run against its off run. Empty means equivalent.
pub fn compare_runs(off: &RunRecord, on: &RunRecord) -> Vec<Mismatch> {
    let mut mismatches = Vec::new();
    let mut differ = |what: String, off: String, on: String| {
        mismatches.push(Mismatch { what, off, on });
    };
    if off.error != on.error {
        differ(
            "error".into(),
            format!("{:?}", off.error),
            format!("{:?}", on.error),
        );
    }
    if off.steps.len() != on.steps.len() {
        differ(
            "commands run".into(),
            off.steps.len().to_string(),
            on.steps.len().to_string(),
        );
    }
    // The first step that parts is the one worth reading; the rest follow
    // from it.
    if let Some((a, b)) = off.steps.iter().zip(&on.steps).find(|(a, b)| a != b) {
        differ(
            format!("step {} `{}`", a.index + 1, a.command),
            a.digest.to_string(),
            format!("`{}` {}", b.command, b.digest),
        );
    }
    if off.final_digest != on.final_digest {
        differ(
            "final digest".into(),
            format!("{:?}", off.final_digest.map(|d| d.to_string())),
            format!("{:?}", on.final_digest.map(|d| d.to_string())),
        );
    }
    if off.final_state_line != on.final_state_line {
        differ(
            "final state line".into(),
            off.final_state_line.clone(),
            on.final_state_line.clone(),
        );
    }
    if off.final_frame_hash != on.final_frame_hash {
        differ(
            "final frame".into(),
            format!("{:?}", off.final_frame_hash),
            format!("{:?}", on.final_frame_hash),
        );
    }
    if off.save_error != on.save_error {
        differ(
            "scripted save".into(),
            format!("{:?}", off.save_error),
            format!("{:?}", on.save_error),
        );
    }
    for ((name, a), (_, b)) in off.saves.iter().zip(&on.saves) {
        if a != b {
            differ(
                format!("saved {name}"),
                format!("{a:x?}"),
                format!("{b:x?}"),
            );
        }
    }
    mismatches
}

/// Runs `commands` on `state` with `hooks`, recording the digest after each
/// command, then saves with `Q`, `Y` into `save_dir` when one is given.
pub fn run_trace(
    mut state: PlayState,
    game_dir: &Path,
    commands: &[String],
    hooks: &mut dyn ScriptTokenHooks,
    save_dir: Option<&Path>,
) -> RunRecord {
    let mut record = RunRecord::default();
    let result = replay_play_script_commands_hooked(
        &mut state,
        game_dir,
        commands,
        hooks,
        |state, index, command| {
            record.steps.push(TraceStep {
                index,
                command: command.to_string(),
                digest: trajectory_digest(state),
            });
            Ok(())
        },
    );
    if let Err(error) = result {
        record.error = Some(error.to_string());
        return record;
    }
    finish_record(&mut record, &mut state, save_dir);
    record
}

/// The final digest and state line, then the scripted save.
fn finish_record(record: &mut RunRecord, state: &mut PlayState, save_dir: Option<&Path>) {
    record.final_digest = Some(trajectory_digest(state));
    record.final_state_line = play_script_state_line(state);
    if let Some(dir) = save_dir {
        match scripted_save(state, dir) {
            Ok(saves) => record.saves = saves,
            Err(error) => record.save_error = Some(error.to_string()),
        }
    }
}

/// `Q`, `Y` into `dir`, then the hash of each of [`SAVE_FILES`] there.
pub fn scripted_save(
    state: &mut PlayState,
    dir: &Path,
) -> io::Result<Vec<(&'static str, Option<u64>)>> {
    handle_play_key_input(state, 'Q', "", dir)?;
    handle_play_key_input(state, 'Y', "", dir)?;
    Ok(SAVE_FILES
        .iter()
        .map(|name| (*name, fs::read(dir.join(name)).ok().map(|b| fnv1a64(&b))))
        .collect())
}

/// Runs a script twice from one state, with `features` on and then all off
/// with their events removed, and compares the two runs (design 1.3.12).
/// `save_dirs`, when given, are two scratch folders holding the same seed
/// save, one per run.
pub fn check_equivalence(
    state: &PlayState,
    game_dir: &Path,
    commands: &[String],
    features: Vec<Box<dyn TerminalFeature>>,
    save_dirs: Option<(&Path, &Path)>,
) -> Vec<Mismatch> {
    let mut on_hooks = FeatureHooks::new(features);
    let on = run_trace(
        state.clone(),
        game_dir,
        commands,
        &mut on_hooks,
        save_dirs.map(|dirs| dirs.1),
    );
    let mut off_hooks = ReplayHooks::new(on_hooks.events());
    let off = run_trace(
        state.clone(),
        game_dir,
        commands,
        &mut off_hooks,
        save_dirs.map(|dirs| dirs.0),
    );
    compare_runs(&off, &on)
}

/// Parses a matrix file: one row per line, `NAME<TAB>SETTINGS`, where
/// SETTINGS is a comma-separated list of `preset=NAME` and `KEY=VALUE`
/// (the value as `--set` takes it). `#` starts a comment line. A row named
/// `classic` is refused: the suite always runs Classic itself.
pub fn parse_matrix(text: &str) -> io::Result<Vec<MatrixRow>> {
    let invalid = |line: usize, message: String| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("matrix line {line}: {message}"),
        )
    };
    let mut rows: Vec<MatrixRow> = Vec::new();
    for (number, line) in text.lines().enumerate() {
        let number = number + 1;
        let line = line.trim_end();
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        let (name, list) = line.split_once('\t').unwrap_or((line, ""));
        let name = name.trim();
        if name.is_empty() || name.contains(char::is_whitespace) {
            return Err(invalid(number, format!("bad row name `{name}`")));
        }
        if name == CLASSIC_RUN || rows.iter().any(|row| row.name == name) {
            return Err(invalid(number, format!("row `{name}` is listed twice")));
        }
        let mut settings = Vec::new();
        for item in list.split(',').map(str::trim).filter(|i| !i.is_empty()) {
            let Some((key, value)) = item.split_once('=') else {
                return Err(invalid(number, format!("`{item}` is not KEY=VALUE")));
            };
            let (key, value) = (key.trim(), value.trim());
            if key == "preset" {
                let preset = Preset::from_key(value)
                    .ok_or_else(|| invalid(number, format!("unknown preset `{value}`")))?;
                settings.push(CliSetting::Preset(preset));
            } else {
                check_cli_key("--matrix", key, false)
                    .map_err(|error| invalid(number, error.to_string()))?;
                settings.push(CliSetting::Set {
                    key: key.to_string(),
                    value: value.to_string(),
                });
            }
        }
        rows.push(MatrixRow {
            name: name.to_string(),
            settings,
        });
    }
    Ok(rows)
}

/// What a route-smoke case run recorded for the suite.
struct RouteRecorder<'a> {
    seed_dir: &'a Path,
    case_name: &'a str,
    label: &'a str,
    record: RunRecord,
}

impl RouteTraceObserver for RouteRecorder<'_> {
    fn after_command(&mut self, state: &PlayState, index: usize, command: &str) {
        self.record.steps.push(TraceStep {
            index,
            command: command.to_string(),
            digest: trajectory_digest(state),
        });
    }

    fn finished(&mut self, state: &mut PlayState, _command_game_dir: &Path) -> io::Result<()> {
        let dir = route_smoke_temp_dir(self.case_name, self.label)?;
        let seeded = seed_route_smoke_save_files(self.seed_dir, &dir);
        let mut record = std::mem::take(&mut self.record);
        match seeded {
            Ok(()) => finish_record(&mut record, state, Some(&dir)),
            Err(error) => record.error = Some(format!("seed save: {error}")),
        }
        self.record = record;
        let _ = fs::remove_dir_all(&dir);
        Ok(())
    }
}

/// One route-smoke case under one set of hooks.
fn run_route_case(
    game_dir: &Path,
    atlas: &u5_runtime::TileAtlas,
    case: &crate::route_smoke::RouteSmokeCase,
    hooks: &mut dyn ScriptTokenHooks,
    label: &str,
) -> io::Result<(RunRecord, Option<RouteSmokeReport>)> {
    // The cases write the two object files into the game folder; every run
    // starts from the same ones, as the route smoke does.
    let brit = fs::read(game_dir.join(BRIT_OOL_FILENAME))?;
    let under = fs::read(game_dir.join(UNDER_OOL_FILENAME))?;
    let mut recorder = RouteRecorder {
        seed_dir: game_dir,
        case_name: case.name,
        label,
        record: RunRecord::default(),
    };
    let result = run_route_smoke_case_traced(game_dir, atlas, case, hooks, &mut recorder);
    fs::write(game_dir.join(BRIT_OOL_FILENAME), &brit)?;
    fs::write(game_dir.join(UNDER_OOL_FILENAME), &under)?;
    let mut record = recorder.record;
    let report = match result {
        Ok(report) => {
            record.final_frame_hash = Some(report.final_hash);
            Some(report)
        }
        Err(error) => {
            record.error = Some(error.to_string());
            None
        }
    };
    Ok((record, report))
}

/// One report line; tabs and line breaks in a field become spaces.
fn report_line(
    case: &str,
    configuration: &str,
    result: &str,
    steps: usize,
    saved: usize,
    digest: &str,
    detail: &str,
) -> String {
    [
        case,
        configuration,
        result,
        &steps.to_string(),
        &saved.to_string(),
        digest,
        detail,
    ]
    .map(|field| field.replace(['\t', '\r', '\n'], " "))
    .join("\t")
}

/// For each configuration, the index of the first earlier one that resolves
/// exactly as it does (same toggles, values, key bindings and lock), or
/// `None` for the first of its kind. Every run is a function of the case
/// and the resolved settings alone, so a configuration that resolves as an
/// earlier one runs the same runs: the suite runs each distinct resolution
/// once. Until a row's feature ships it resolves as Classic (U13).
pub fn plan_configurations(configs: &[(String, ResolvedEnhancements)]) -> Vec<Option<usize>> {
    let same = |a: &ResolvedEnhancements, b: &ResolvedEnhancements| {
        a.toggles == b.toggles && a.values == b.values && a.keys == b.keys && a.lock == b.lock
    };
    configs
        .iter()
        .enumerate()
        .map(|(index, (_, resolved))| {
            configs[..index]
                .iter()
                .position(|(_, earlier)| same(earlier, resolved))
        })
        .collect()
}

/// What a configuration resolved to, for the report's header: the toggles
/// on, how many settings were refused or not yet shipped, and the
/// configuration it runs as.
fn configuration_summary(resolved: &ResolvedEnhancements, same_as: Option<&str>) -> String {
    let on: Vec<&str> = resolved.toggles.iter_on().map(Toggle::key).collect();
    let mut summary = if on.is_empty() {
        "no toggle on".to_string()
    } else {
        format!("on {}", on.join(" "))
    };
    if !resolved.notes.is_empty() {
        summary.push_str(&format!("; {} note(s)", resolved.notes.len()));
    }
    if let Some(first) = same_as {
        summary.push_str(&format!("; runs as {first}"));
    }
    summary
}

/// The terminal features of a gem consumption case's on run: as
/// [`terminal_features`], except that the gem map, when it is on, also
/// draws the map a window would show each time a gem view opens, with the
/// configuration's own `[gem_map]` keys and the real files, into `scratch`
/// (removed after the suite). So the cases prove the renderer, too, leaves
/// the game alone.
pub fn gem_case_features(
    resolved: &ResolvedEnhancements,
    game_dir: &Path,
    scratch: &Path,
) -> Vec<Box<dyn TerminalFeature>> {
    terminal_features(resolved)
        .into_iter()
        .map(|feature| -> Box<dyn TerminalFeature> {
            if feature.toggle() != Toggle::GemMapEnhanced {
                return feature;
            }
            let png = GemPng::new(
                scratch,
                gem_canvas_16_9(),
                game_dir,
                TileGraphicsDepth::Ega16,
                resolved.gem_map(),
                resolved.display_map_brightness(),
            );
            Box::new(TerminalGemMap::in_memory().with_png(png))
        })
        .collect()
}

/// The 16:9 gem canvas (512x240), where the suite's pictures are drawn.
fn gem_canvas_16_9() -> u5_automap::gem::GemCanvas {
    u5_automap::gem::GemCanvas { w: 512, h: 240 }
}

/// `u5-engine --equivalence-suite OUT [--matrix FILE] GAME_DIR`.
///
/// For every route-smoke case, and then every gem consumption case
/// ([`crate::gem_equivalence`]): the all-off run, then Classic and each
/// matrix row with that row's features on, each compared against the
/// all-off run with the row's own events removed. A row that resolves
/// exactly as an earlier one shares its runs ([`plan_configurations`]).
/// Writes [`REPORT_FILE`] and [`CLASSIC_MANIFEST_FILE`] into `out_dir` and
/// fails on any difference.
pub fn run_equivalence_suite(
    game_dir: &Path,
    raster_depth: TileGraphicsDepth,
    out_dir: &Path,
    matrix: Option<&Path>,
) -> io::Result<()> {
    let mut rows = vec![MatrixRow {
        name: CLASSIC_RUN.to_string(),
        settings: vec![CliSetting::Preset(Preset::Classic)],
    }];
    if let Some(path) = matrix {
        rows.extend(parse_matrix(&fs::read_to_string(path)?)?);
    }
    let configs: Vec<(String, ResolvedEnhancements)> = rows
        .into_iter()
        .map(|row| {
            let resolved = resolve_launch(None, &row.settings, LockState::Off).resolved;
            (row.name, resolved)
        })
        .collect();
    let plan = plan_configurations(&configs);

    let cases = route_smoke_cases();
    let atlas = load_tile_atlas(game_dir, raster_depth)?;
    fs::create_dir_all(out_dir)?;
    let distinct = plan.iter().filter(|same| same.is_none()).count();
    println!(
        "Equivalence suite: {} case(s) x {} configuration(s), {distinct} distinct.",
        cases.len(),
        configs.len()
    );
    let mut lines = vec![
        "# Ultima V enhancement equivalence report".to_string(),
        "# Sanitized: route names, configuration names and hashes only.".to_string(),
    ];
    for ((name, resolved), same) in configs.iter().zip(&plan) {
        lines.push(format!(
            "# configuration {name}: {}",
            configuration_summary(resolved, same.map(|index| configs[index].0.as_str()))
        ));
    }
    lines.push(
        "# case\tconfiguration\tresult\tsteps\tsaved-files\tfinal-digest\tdetail".to_string(),
    );
    let mut classic_reports = Vec::with_capacity(cases.len());
    let mut failures = Vec::new();
    for case in &cases {
        let (baseline, _) =
            run_route_case(game_dir, &atlas, case, &mut NoScriptTokenHooks, "eq-off")?;
        // Each distinct configuration's result for this case, by index.
        let mut results: BTreeMap<usize, (&'static str, String, usize, usize, String)> =
            BTreeMap::new();
        for (index, ((name, resolved), same)) in configs.iter().zip(&plan).enumerate() {
            if let Some(first) = *same {
                // Resolves exactly as an earlier configuration: the runs are
                // the same runs, so its result stands for this one.
                let (result, detail, steps, saved, digest) = results[&first].clone();
                println!(
                    "equivalence {} {name}: {result} (as {})",
                    case.name, configs[first].0
                );
                let detail = if detail.is_empty() {
                    format!("as {}", configs[first].0)
                } else {
                    format!("as {}: {detail}", configs[first].0)
                };
                lines.push(report_line(
                    case.name, name, result, steps, saved, &digest, &detail,
                ));
                continue;
            }
            let mut hooks = FeatureHooks::new(terminal_features(resolved));
            let (on, report) = run_route_case(game_dir, &atlas, case, &mut hooks, "eq-on")?;
            let events = hooks.into_events();
            // With no token taken the off run is the baseline itself.
            let off = if events.is_empty() {
                baseline.clone()
            } else {
                let mut replay = ReplayHooks::new(&events);
                run_route_case(game_dir, &atlas, case, &mut replay, "eq-replay")?.0
            };
            let mismatches = compare_runs(&off, &on);
            if name == CLASSIC_RUN {
                match report {
                    Some(report) => classic_reports.push(report),
                    None => failures.push(format!(
                        "{} classic: route smoke failed: {}",
                        case.name,
                        on.error.clone().unwrap_or_default()
                    )),
                }
            }
            let final_digest = on
                .final_digest
                .map(|d| d.to_string())
                .unwrap_or_else(|| "-".into());
            let (result, detail) =
                if let Some(error) = on.error.as_ref().filter(|_| mismatches.is_empty()) {
                    // Equal, but a run that failed compared less than it should:
                    // the suite cannot vouch for it.
                    failures.push(format!("{} {name}: both runs failed: {error}", case.name));
                    ("FAILED", error.clone())
                } else if mismatches.is_empty() {
                    ("equal", String::new())
                } else {
                    let detail = mismatches
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join("; ");
                    failures.push(format!("{} {name}: {detail}", case.name));
                    ("DIFFER", detail)
                };
            println!("equivalence {} {name}: {result}", case.name);
            let steps = on.steps.len();
            let saved = on.saves.iter().filter(|(_, hash)| hash.is_some()).count();
            lines.push(report_line(
                case.name,
                name,
                result,
                steps,
                saved,
                &final_digest,
                &detail,
            ));
            results.insert(index, (result, detail, steps, saved, final_digest));
        }
    }
    // The gem consumption cases (design 10.6, M3-18): not route-smoke
    // cases, so Classic's manifest and its digest are untouched.
    let gem_cases = gem_consumption_cases();
    let gem_scratch = route_smoke_temp_dir("gem-cases", "png")?;
    for case in &gem_cases {
        let baseline = run_gem_case(game_dir, case, &mut NoScriptTokenHooks, "gem-off")?;
        let mut results: BTreeMap<usize, (&'static str, String, usize)> = BTreeMap::new();
        for (index, ((name, resolved), same)) in configs.iter().zip(&plan).enumerate() {
            if let Some(first) = *same {
                let (result, detail, steps) = results[&first].clone();
                let detail = if detail.is_empty() {
                    format!("as {}", configs[first].0)
                } else {
                    format!("as {}: {detail}", configs[first].0)
                };
                lines.push(report_line(case.name, name, result, steps, 0, "-", &detail));
                continue;
            }
            let mut hooks = FeatureHooks::new(gem_case_features(resolved, game_dir, &gem_scratch));
            let on = run_gem_case(game_dir, case, &mut hooks, "gem-on")?;
            let events = hooks.into_events();
            let off = if events.is_empty() {
                baseline.clone()
            } else {
                let mut replay = ReplayHooks::new(&events);
                run_gem_case(game_dir, case, &mut replay, "gem-replay")?
            };
            let mut problems: Vec<String> = compare_gem_runs(&off, &on)
                .iter()
                .map(ToString::to_string)
                .collect();
            for (run, which) in [(&off, "off"), (&on, "on")] {
                if let Err(why) = check_gem_expectation(case, run) {
                    problems.push(format!("{which}: not as 1.0.1: {why}"));
                }
            }
            let (result, detail) = if problems.is_empty() {
                ("equal", String::new())
            } else {
                let detail = problems.join("; ");
                failures.push(format!("{} {name}: {detail}", case.name));
                ("DIFFER", detail)
            };
            println!("equivalence {} {name}: {result}", case.name);
            let steps = on.beats.len().saturating_sub(1);
            let saved = on.saves.iter().filter(|(_, hash)| hash.is_some()).count();
            let digest = on
                .beats
                .last()
                .map_or_else(|| "-".into(), |beat| beat.digest.to_string());
            lines.push(report_line(
                case.name, name, result, steps, saved, &digest, &detail,
            ));
            results.insert(index, (result, detail, steps));
        }
    }
    let _ = fs::remove_dir_all(&gem_scratch);
    let report_path = out_dir.join(REPORT_FILE);
    let mut report = fs::File::create(&report_path)?;
    for line in &lines {
        writeln!(report, "{line}")?;
    }
    println!("Saved equivalence report: {}.", report_path.display());
    if classic_reports.len() == cases.len() {
        let manifest = out_dir.join(CLASSIC_MANIFEST_FILE);
        write_route_smoke_manifest(&manifest, &classic_reports)?;
        println!(
            "Saved Classic route smoke manifest: {}.",
            manifest.display()
        );
    }
    if !failures.is_empty() {
        println!("\nEquivalence suite: {} difference(s).", failures.len());
        for failure in &failures {
            println!("  {failure}");
        }
        return Err(io::Error::other(format!(
            "{} equivalence difference(s)",
            failures.len()
        )));
    }
    println!("Equivalence suite: every configuration equals all-off.");
    Ok(())
}

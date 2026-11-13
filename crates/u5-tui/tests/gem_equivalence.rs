//! The gem consumption cases of the equivalence suite (Phase B design 10.6;
//! build plan M3-18): the committed case list, the per-command comparison
//! (gem count, view overlay, idle polls, prng, clock, transcript, position,
//! digest) and the save bytes, the 1.0.1 expectation of each case, and the
//! harness catching a feature that spends a gem, holds the view or lets the
//! dismissal key through. On synthetic states in every `cargo test`; the
//! five real cases when `U5_TEST_ASSET_DIR` names a writable copy.

use std::fs;
use std::path::{Path, PathBuf};

use u5_runtime::enhancements::{
    CliSetting, EffectClass, HostCapabilities, LockState, ResolveInput, ResolvedEnhancements,
    Toggle, resolve,
};
use u5_runtime::test_fixtures::{
    configured_original_asset_dir, dungeon_state, open_dungeon_record, open_grid, open_world_grid,
    saved_game_seed_bytes, synthetic_tile_atlas, test_state, workspace_file, world_state,
    write_empty_ool_mirrors,
};
use u5_runtime::{Area, PlayState, SAVED_GAM_FILENAME, TileGraphicsDepth, WorldPlane};
use u5_tui::*;

const BLESS: &str = "U5_BLESS_GEM_CASES";

fn script(commands: &[&str]) -> Vec<String> {
    commands.iter().map(|c| (*c).to_string()).collect()
}

/// A folder holding a seed save, removed on drop.
struct Scratch(PathBuf);

impl Scratch {
    fn new(label: &str) -> Self {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "u5-tui-gem-equivalence-{label}-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join(SAVED_GAM_FILENAME),
            saved_game_seed_bytes(0x01, 0, 5, 5),
        )
        .unwrap();
        write_empty_ool_mirrors(&dir);
        Self(dir)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

// ---------------------------------------------------------------------------
// The committed list.

#[test]
fn the_committed_case_list_is_the_generators() {
    let path = workspace_file(GEM_CASES_FILE);
    let expected = gem_cases_tsv(&gem_consumption_cases());
    if std::env::var_os(BLESS).is_some_and(|v| v == "1") {
        fs::write(&path, &expected).unwrap();
    }
    let text = fs::read_to_string(&path).expect("the committed gem case list");
    assert_eq!(
        text, expected,
        "{GEM_CASES_FILE} is out of date ({BLESS}=1)"
    );
}

#[test]
fn the_five_cases_of_10_6_are_there_and_none_is_a_route_case() {
    let cases = gem_consumption_cases();
    let places: Vec<(GemPlace, u8, GemExpect)> =
        cases.iter().map(|c| (c.place, c.gems, c.expect)).collect();
    assert_eq!(
        places,
        [
            (GemPlace::Outdoors, 2, GemExpect::Opens),
            (GemPlace::Town, 2, GemExpect::Opens),
            (GemPlace::Dungeon, 2, GemExpect::Opens),
            (GemPlace::Outdoors, 0, GemExpect::Nothing),
            (GemPlace::Combat, 2, GemExpect::Nothing),
        ]
    );
    let routes: Vec<&str> = route_smoke_cases().iter().map(|c| c.name).collect();
    for case in &cases {
        assert_eq!(case.script[0], "v", "{}", case.name);
        assert!(case.script.len() >= 2, "{}: a dismissal key", case.name);
        assert!(
            !routes.contains(&case.name),
            "{} is a route case",
            case.name
        );
    }
}

// ---------------------------------------------------------------------------
// Synthetic states.

fn outdoors(gems: u8) -> PlayState {
    let mut state = world_state(open_world_grid(), 100, 100);
    state.area = Area::World {
        plane: WorldPlane::Britannia,
    };
    state.gems = gems;
    state
}

fn town(gems: u8) -> PlayState {
    let mut state = test_state(open_grid(), 5, 5);
    state.gems = gems;
    state
}

fn dungeon(gems: u8) -> PlayState {
    let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
    state.gems = gems;
    state
}

fn resolved(cli: &[CliSetting]) -> ResolvedEnhancements {
    resolve(ResolveInput {
        stored: None,
        cli,
        lock: LockState::Off,
        caps: HostCapabilities::ALL,
    })
}

fn gem_on() -> ResolvedEnhancements {
    resolved(&[CliSetting::Enable("gem_map.enhanced".into())])
}

/// The gem map with its picture drawn on every open, on the synthetic
/// atlas, into `dir`.
fn drawing_gem_map(dir: &Path) -> Box<dyn TerminalFeature> {
    let png = GemPng::terminal(
        dir,
        u5_automap::gem::GemCanvas { w: 512, h: 240 },
        dir,
        TileGraphicsDepth::Ega16,
    )
    .with_art(u5_automap::gem::MapArt::build(&synthetic_tile_atlas(
        TileGraphicsDepth::Ega16,
    )));
    Box::new(TerminalGemMap::in_memory().with_png(png))
}

/// The case `case` from `state`, on with `on` and off, saved in two
/// scratch folders: the two runs and their differences.
fn on_and_off(
    state: &PlayState,
    commands: &[&str],
    on: Vec<Box<dyn TerminalFeature>>,
) -> (GemRun, GemRun, Vec<Mismatch>) {
    let (off_dir, on_dir) = (Scratch::new("off"), Scratch::new("on"));
    let commands = script(commands);
    let mut hooks = FeatureHooks::new(on);
    let on = run_gem_commands(
        state.clone(),
        on_dir.path(),
        &commands,
        &mut hooks,
        Some(on_dir.path()),
    );
    let mut replay = ReplayHooks::new(hooks.events());
    let off = run_gem_commands(
        state.clone(),
        off_dir.path(),
        &commands,
        &mut replay,
        Some(off_dir.path()),
    );
    let mismatches = compare_gem_runs(&off, &on);
    (off, on, mismatches)
}

fn case(
    name: &'static str,
    gems: u8,
    script: &'static [&'static str],
    expect: GemExpect,
) -> GemCase {
    GemCase {
        name,
        place: GemPlace::Outdoors,
        gems,
        script,
        expect,
    }
}

#[test]
fn a_gem_outdoors_in_a_town_and_in_a_dungeon_is_spent_as_off() {
    let shots = Scratch::new("shots");
    for (name, state, commands) in [
        ("outdoors", outdoors(2), &["v", "s", "a", "pass"][..]),
        ("town", town(2), &["v", "s", "a", "pass"][..]),
        ("dungeon", dungeon(2), &["v", "s", "pass"][..]),
    ] {
        let (off, on, mismatches) =
            on_and_off(&state, commands, vec![drawing_gem_map(shots.path())]);
        assert_eq!(mismatches, [], "{name}");
        let expected = case("synthetic", 2, &["v", "s"], GemExpect::Opens);
        for run in [&off, &on] {
            check_gem_expectation(&expected, run).unwrap_or_else(|why| panic!("{name}: {why}"));
            assert!(
                run.saves.iter().any(|(_, hash)| hash.is_some()),
                "{name}: the scripted save wrote"
            );
        }
        assert!(on.beats[1].gem_view && on.beats[1].gems == 1, "{name}");
    }
    // The map was really drawn: one picture per open (three opens).
    let written = fs::read_dir(shots.path())
        .unwrap()
        .filter(|entry| {
            entry
                .as_ref()
                .is_ok_and(|e| e.path().extension().is_some_and(|x| x == "png"))
        })
        .count();
    assert_eq!(written, 3);
}

#[test]
fn a_v_with_no_gem_opens_nothing_and_spends_nothing_on_and_off() {
    let (off, on, mismatches) = on_and_off(
        &outdoors(0),
        &["v", "s", "a", "pass"],
        terminal_features(&gem_on()),
    );
    assert_eq!(mismatches, []);
    let expected = case("none", 0, &["v", "s"], GemExpect::Nothing);
    for run in [&off, &on] {
        check_gem_expectation(&expected, run).unwrap();
    }
}

#[test]
fn every_part_is_compared_per_command_and_the_start_too() {
    let (off, on, mismatches) = on_and_off(&outdoors(2), &["v", "s", "pass"], Vec::new());
    assert_eq!(mismatches, []);
    assert_eq!(on.beats.len(), 4, "the start and three commands");
    assert_eq!(on.beats[0].command, "start");
    // Each part, changed at one beat, is reported at that beat by name.
    type Edit = fn(&mut GemBeat);
    let edits: [(&str, Edit); 7] = [
        ("gem count", |b| b.gems += 1),
        ("view overlay", |b| b.overlay.push('!')),
        ("idle polls", |b| b.idle_polls += 1),
        ("prng", |b| b.prng ^= 1),
        ("clock", |b| b.clock.push('!')),
        ("transcript", |b| b.transcript ^= 1),
        ("position", |b| b.position.push('!')),
    ];
    for (what, edit) in edits {
        let mut changed = on.clone();
        edit(&mut changed.beats[2]);
        let found = compare_gem_runs(&off, &changed);
        assert_eq!(found.len(), 1, "{what}: {found:?}");
        assert_eq!(found[0].what, format!("step 2 `s` {what}"));
    }
    let mut saved = on.clone();
    saved.saves[0].1 = Some(0);
    assert!(compare_gem_runs(&off, &saved)[0].what.starts_with("saved "));
}

/// A feature that spends a gem when it sees the view open: the gem count
/// differs from the step it acted on.
struct GemThief;

impl TerminalFeature for GemThief {
    fn toggle(&self) -> Toggle {
        Toggle::GemMapEnhanced
    }
    fn class(&self) -> EffectClass {
        EffectClass::Presentation
    }
    fn owns_token(&self, token: &str) -> bool {
        token == "@thief"
    }
    fn on_token(&mut self, _token: &str, state: &PlayState) -> Vec<String> {
        // Presentation in name only: it types a second `v`.
        if state.gems > 0 {
            vec!["v".into()]
        } else {
            Vec::new()
        }
    }
}

#[test]
fn a_feature_that_spends_a_gem_is_caught_and_breaks_the_expectation() {
    // Declared presentation, so the off run drops the `v` it typed.
    let (_, on, found) = on_and_off(
        &outdoors(2),
        &["v", "s", "@thief", "pass"],
        vec![Box::new(GemThief)],
    );
    assert!(
        found.iter().any(|m| m.what.ends_with("gem count")),
        "{found:?}"
    );
    let expected = case("thief", 2, &["v", "s"], GemExpect::Opens);
    assert!(check_gem_expectation(&expected, &on).is_err());
}

#[test]
fn the_expectation_catches_a_held_view_and_a_key_let_through() {
    let state = outdoors(2);
    let (_, on, _) = on_and_off(&state, &["v", "s", "pass"], Vec::new());
    let expected = case("x", 2, &["v", "s"], GemExpect::Opens);
    check_gem_expectation(&expected, &on).unwrap();
    // The view still up after the dismissal key.
    let mut held = on.clone();
    held.beats[2].gem_view = true;
    assert!(
        check_gem_expectation(&expected, &held)
            .unwrap_err()
            .contains("left the view up")
    );
    // The dismissal key moved the party.
    let mut through = on.clone();
    through.beats[2].position.push_str(" moved");
    assert!(
        check_gem_expectation(&expected, &through)
            .unwrap_err()
            .contains("reached the game")
    );
    // Nothing must open: a gem view at any beat fails.
    let nothing = case("y", 2, &["v", "s"], GemExpect::Nothing);
    assert!(check_gem_expectation(&nothing, &on).is_err());
    assert!(on.beats[1].overlay.contains("GemView"));
}

// ---------------------------------------------------------------------------
// Real game files.

/// A writable copy of the game, removed on drop.
fn real_copy(dir: &Path, label: &str) -> Scratch {
    let scratch = Scratch::new(label);
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            fs::copy(entry.path(), scratch.path().join(entry.file_name())).unwrap();
        }
    }
    scratch
}

/// The five cases on the real files, the gem map on with every `[gem_map]`
/// key at its default and with "all on except cheats", against the off
/// run: equal per command and in the save, and each as 1.0.1 has it.
#[test]
fn the_five_cases_on_real_files() {
    let Some(dir) = configured_original_asset_dir() else {
        return;
    };
    if !dir.join("BRIT.DAT").exists() {
        return;
    }
    let game = real_copy(&dir, "real");
    let shots = Scratch::new("real-shots");
    let all_on = resolved(&[
        CliSetting::Enable("gem_map.enhanced".into()),
        CliSetting::Enable("display.grow_left".into()),
        CliSetting::Enable("display.sprite_composite".into()),
        CliSetting::Enable("enhancements.freeze_overlays".into()),
    ]);
    for configuration in [gem_on(), all_on] {
        for case in gem_consumption_cases() {
            let start = gem_case_state(game.path(), &case).unwrap();
            assert_eq!(
                start.combat_frame_snapshot.is_some(),
                case.place == GemPlace::Combat,
                "{}: in combat only in the combat case",
                case.name
            );
            let off = run_gem_case(game.path(), &case, &mut NoScriptTokenHooks, "t-off").unwrap();
            let mut hooks = FeatureHooks::new(equivalence::gem_case_features(
                &configuration,
                game.path(),
                shots.path(),
            ));
            let on = run_gem_case(game.path(), &case, &mut hooks, "t-on").unwrap();
            assert_eq!(compare_gem_runs(&off, &on), [], "{}", case.name);
            for run in [&off, &on] {
                check_gem_expectation(&case, run)
                    .unwrap_or_else(|why| panic!("{}: {why}", case.name));
            }
        }
    }
    // Outdoors, the town and the dungeon drew their maps, twice each.
    let written = fs::read_dir(shots.path()).unwrap().count();
    assert_eq!(written, 6);
}

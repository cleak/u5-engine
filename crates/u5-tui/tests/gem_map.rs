//! The gem map's terminal half (Phase B design 10.3.6, 10.6; plan M3-08).
//!
//! - One scripted scenario per event row of 10.3.3, each asserting by
//!   count and key (never text) that its command records exactly its
//!   discovery. Most run a route-smoke case's own script on the player's
//!   files (`U5_TEST_ASSET_DIR`, a `test-assets` copy); the rows no route
//!   case reaches (a named entrance, a fall, the Search finds, a fountain)
//!   run a short play script on a synthetic place with the player's files
//!   behind the dispatcher. Each skips without the files.
//! - The record's file: written only after a save, an older save's record
//!   restored with it, OFF opening no store and leaving an old file's bytes
//!   and modification time alone, and `SAVED.GAM` equal on and off.
//! - The flag and the lock: `--gem-map` parses for the play and route modes
//!   only, the terminal feature is absent under Classic and the lock, the
//!   gem map is equivalent (class a) and `@gemmap` is consumed only while
//!   it is on.

use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, SystemTime};

use u5_automap::gem::{FeatureKey, GemKnowledge, GemSession};
use u5_runtime::enhancements::{
    CliSetting, EffectClass, GEM_G2, HostCapabilities, LockReason, LockState, ResolveInput,
    SIDECAR_DIR, Toggle, resolve_as_shipped,
};
use u5_runtime::test_fixtures::{
    configured_original_asset_dir, dungeon_state, open_dungeon_record, open_grid, open_world_grid,
    saved_game_seed_bytes, test_state, world_state, write_empty_ool_mirrors,
};
use u5_runtime::{
    Area, Direction, DungeonLookFocus, PlayState, SAVED_GAM_FILENAME, TileGraphicsDepth,
    WORD_OF_POWER_SEALED_TILE, WorldPlane, dungeon_cell_index, load_tile_atlas, world_cell_index,
};
use u5_tui::*;

fn script(commands: &[&str]) -> Vec<String> {
    commands.iter().map(|c| (*c).to_string()).collect()
}

// ---------------------------------------------------------------------------
// The record by kind: counts only.

/// What a record holds, kind by kind.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Counts {
    places: usize,
    shrines: u32,
    underworld: bool,
    codex: bool,
    entrances: u32,
    gates: usize,
    signs: usize,
    trades: usize,
    keepers: usize,
    links: usize,
    exits: usize,
    marks: usize,
    cells: u32,
    sides: u32,
}

impl Counts {
    fn of(know: &GemKnowledge) -> Self {
        let [shrines, _, named] = know.flag_bytes();
        Self {
            places: know.entered().len(),
            shrines: shrines.count_ones(),
            underworld: know.is_known(&FeatureKey::Underworld),
            codex: know.is_known(&FeatureKey::Codex),
            entrances: named.count_ones(),
            gates: know.gates().len(),
            signs: know.signs().len(),
            trades: know.trades().len(),
            keepers: know.keepers().len(),
            links: know.town_links().len(),
            exits: know.exits().len(),
            marks: know.marks().len(),
            cells: know.cells().total(),
            sides: know
                .sides()
                .values()
                .map(|bits| bits.iter().map(|b| b.count_ones()).sum::<u32>())
                .sum(),
        }
    }

    /// The kinds that differ from `before`, by name.
    fn changed_from(&self, before: &Self) -> Vec<&'static str> {
        let mut kinds = Vec::new();
        let mut check = |name, changed: bool| {
            if changed {
                kinds.push(name);
            }
        };
        check("places", self.places != before.places);
        check("shrines", self.shrines != before.shrines);
        check("underworld", self.underworld != before.underworld);
        check("codex", self.codex != before.codex);
        check("entrances", self.entrances != before.entrances);
        check("gates", self.gates != before.gates);
        check("signs", self.signs != before.signs);
        check("trades", self.trades != before.trades);
        check("keepers", self.keepers != before.keepers);
        check("links", self.links != before.links);
        check("exits", self.exits != before.exits);
        check("marks", self.marks != before.marks);
        check("cells", self.cells != before.cells);
        check("sides", self.sides != before.sides);
        kinds
    }
}

/// Seen cells come with any step in a dungeon or outdoors; they are never
/// the discovery a row is about unless the row says so.
const SEEN: [&str; 2] = ["cells", "sides"];

/// A class (a) terminal feature that keeps a gem record and logs it after
/// every command, for the scenarios.
struct Recorder {
    session: GemSession,
    log: Rc<RefCell<Vec<(String, GemKnowledge)>>>,
}

impl TerminalFeature for Recorder {
    fn toggle(&self) -> Toggle {
        Toggle::GemMapEnhanced
    }
    fn class(&self) -> EffectClass {
        EffectClass::Presentation
    }
    fn owns_token(&self, _token: &str) -> bool {
        false
    }
    fn on_token(&mut self, _token: &str, _state: &PlayState) -> Vec<String> {
        Vec::new()
    }
    fn before_command(&mut self, state: &PlayState) {
        if self.log.borrow().is_empty() {
            self.session.attach(state, None);
            self.log
                .borrow_mut()
                .push(("start".into(), self.session.knowledge().clone()));
        }
        self.session.before(state);
    }
    fn after_command(&mut self, state: &PlayState) {
        self.session.after(state, 1);
        self.log
            .borrow_mut()
            .push(("command".into(), self.session.knowledge().clone()));
    }
}

/// The records of a route-smoke case's own run: at the start, then after
/// each command.
fn route_records(dir: &Path, case_name: &str) -> Vec<Counts> {
    let atlas = load_tile_atlas(dir, TileGraphicsDepth::Ega16).unwrap();
    let case = route_smoke_cases()
        .into_iter()
        .find(|case| case.name == case_name)
        .unwrap_or_else(|| panic!("route case {case_name}"));
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut hooks = FeatureHooks::new(vec![Box::new(Recorder {
        session: GemSession::new(),
        log: log.clone(),
    })]);
    run_route_smoke_case_traced(dir, &atlas, &case, &mut hooks, &mut NoRouteTrace)
        .unwrap_or_else(|error| panic!("{case_name}: {error}"));
    log.borrow().iter().map(|(_, k)| Counts::of(k)).collect()
}

/// Asserts that the first command that changes `kind` changes nothing else
/// but seen cells (and the places in `also`), and that the run ends with
/// `kind` as `expect` says.
fn assert_row(
    name: &str,
    records: &[Counts],
    kind: &'static str,
    also: &[&'static str],
    expect: impl Fn(&Counts) -> bool,
) {
    let step = records
        .windows(2)
        .position(|pair| pair[1].changed_from(&pair[0]).contains(&kind))
        .unwrap_or_else(|| panic!("{name}: no command recorded {kind}: {records:?}"));
    let changed = records[step + 1].changed_from(&records[step]);
    for other in &changed {
        assert!(
            *other == kind || SEEN.contains(other) || also.contains(other),
            "{name}: command {} also recorded {other}: {changed:?}",
            step + 1
        );
    }
    let last = records.last().unwrap();
    assert!(expect(last), "{name}: {last:?}");
}

fn assets() -> Option<PathBuf> {
    let dir = configured_original_asset_dir()?;
    dir.join("CASTLE.DAT").exists().then_some(dir)
}

#[test]
fn route_rows_record_exactly_their_discovery() {
    let Some(dir) = assets() else {
        eprintln!("skipped: set U5_TEST_ASSET_DIR");
        return;
    };
    // (row, case, kind, kinds the same command may also record, final check)
    type Check = fn(&Counts) -> bool;
    let rows: [(&str, &str, &'static str, &[&'static str], Check); 14] = [
        // Entering by the gate also records that gate (one step in from it).
        (
            "place entered",
            "debug-enter-castle",
            "places",
            &["exits"],
            |c| c.places == 1 && c.exits <= 1,
        ),
        (
            "Underworld visited",
            "britannia-chasm-fall-to-underworld",
            "underworld",
            &[],
            |c| c.underworld && !c.codex,
        ),
        (
            "shrine entered",
            "shrine-native-honesty-meditation",
            "shrines",
            &[],
            |c| c.shrines == 1,
        ),
        (
            "Codex entered",
            "codex-urn-honesty-read",
            "codex",
            &[],
            |c| c.codex && !c.underworld,
        ),
        ("sign read", "yew-wanted-poster-look", "signs", &[], |c| {
            c.signs == 1
        }),
        (
            "shop keeper talked to",
            "shop-tavern-honest-meal-lore-route",
            "trades",
            &["keepers"],
            |c| c.trades == 1,
        ),
        (
            "moongate used",
            "natural-moongate-trammel-gate-travel",
            "gates",
            &["underworld"],
            |c| c.gates == 2,
        ),
        (
            "town ladder or stairs",
            "castle-native-stair-up-route",
            "links",
            &[],
            |c| c.links >= 1,
        ),
        (
            "town exit",
            "castle-canonical-ool-exit",
            "exits",
            &[],
            |c| c.exits == 1,
        ),
        (
            "dungeon ladder",
            "dungeon-ladder-down-up-route",
            "marks",
            &[],
            |c| c.marks == 2,
        ),
        (
            "cell rewrite (chest opened)",
            "dungeon-open-chest-command",
            "marks",
            &[],
            |c| c.marks == 1,
        ),
        (
            "dungeon ladder off the top edge",
            "dungeon-surface-exit-return-world",
            "marks",
            &["sides"],
            |c| c.marks == 1,
        ),
        (
            "dungeon cells seen",
            "dungeon-ignite-torch-route",
            "cells",
            &[],
            |c| c.cells > 1,
        ),
        (
            "town-side cells seen",
            "britannia-move-pass-idle",
            "sides",
            &[],
            |c| c.sides > 0 && c.places == 0,
        ),
    ];
    for (row, case, kind, also, check) in rows {
        let records = route_records(&dir, case);
        assert_row(row, &records, kind, also, check);
    }
}

// ---------------------------------------------------------------------------
// The rows no route case reaches: a play script on a synthetic place, the
// player's files behind the dispatcher.

/// A transcript with some history, so the first Look echo anchors (8.3.4).
fn with_history(mut state: PlayState) -> PlayState {
    for i in 0..3 {
        state.push_message_entry(format!("history {i}"), false);
    }
    state
}

/// Runs `commands` through the terminal play-script loop with a gem
/// session in memory, and returns the record before and after each.
fn scripted_records(state: &mut PlayState, dir: &Path, commands: &[&str]) -> Vec<Counts> {
    let mut records = Vec::new();
    let mut session = TerminalSession {
        save_watch: None,
        gem_map: Some(TerminalGemMap::in_memory()),
    };
    session.start(state);
    records.push(Counts::of(
        session.gem_map.as_ref().unwrap().session().knowledge(),
    ));
    for command in commands {
        run_play_script_commands_watched(state, dir, &script(&[command]), None, Some(&mut session))
            .unwrap();
        records.push(Counts::of(
            session.gem_map.as_ref().unwrap().session().knowledge(),
        ));
    }
    records
}

fn lit_dungeon(level: u8, x: usize, y: usize) -> PlayState {
    let mut state = dungeon_state(open_dungeon_record(), level, x, y);
    state.player.facing = Direction::East;
    state.torch_counter = 50;
    state
}

#[test]
fn scripted_rows_record_exactly_their_discovery() {
    let Some(dir) = assets() else {
        eprintln!("skipped: set U5_TEST_ASSET_DIR");
        return;
    };

    // Entrance named by Look: a sealed entrance at a named dungeon column.
    let mut world = world_state(open_world_grid(), 127, 70);
    world.area = Area::World {
        plane: WorldPlane::Britannia,
    };
    let mut world = with_history(world);
    world.grid[world_cell_index(126, 70)] = WORD_OF_POWER_SEALED_TILE;
    let records = scripted_records(&mut world, &dir, &["l4"]);
    assert_row("entrance named", &records, "entrances", &[], |c| {
        c.entrances == 1 && c.places == 0
    });

    // Fall: a step east onto a hidden fall trap.
    let mut fall = lit_dungeon(1, 2, 5);
    fall.grid[dungeon_cell_index(1, 3, 5)] = 0x69;
    let records = scripted_records(&mut fall, &dir, &["w"]);
    assert_row("fall", &records, "marks", &[], |c| c.marks >= 1);

    // Cell rewrite: a Search that finds the bomb trap underfoot.
    let mut bomb = lit_dungeon(4, 3, 3);
    bomb.grid[dungeon_cell_index(4, 3, 3)] = 0x62;
    let records = scripted_records(&mut bomb, &dir, &["S6"]);
    assert_row("cell rewrite (bomb found)", &records, "marks", &[], |c| {
        c.marks == 1
    });

    // Fountain drunk: the dungeon Look's prompt, answered yes and no.
    for (answer, marked) in [("Y", 1), ("N", 0)] {
        let mut fountain = with_history(lit_dungeon(3, 5, 5));
        fountain.grid[dungeon_cell_index(3, 6, 5)] = 0x50;
        fountain.start_dungeon_fountain_drink_prompt(0, DungeonLookFocus::Ahead);
        let records = scripted_records(&mut fountain, &dir, &[answer]);
        assert_eq!(
            records.last().unwrap().marks,
            marked,
            "fountain answered {answer}"
        );
        if marked == 1 {
            assert_row("fountain drunk", &records, "marks", &[], |c| c.marks == 1);
        }
    }
}

// ---------------------------------------------------------------------------
// The file.

/// A save folder holding a seed save, removed on drop.
struct SaveDir(PathBuf);

impl SaveDir {
    fn new(label: &str) -> Self {
        let unique = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "u5-tui-gem-map-{label}-{}-{unique}",
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

    fn gem_file(&self) -> PathBuf {
        self.0.join(SIDECAR_DIR).join("gemmap.u5e")
    }
}

impl Drop for SaveDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn town() -> PlayState {
    test_state(open_grid(), 3, 4)
}

fn play(state: &mut PlayState, dir: &Path, session: &mut TerminalSession, commands: &[&str]) {
    run_play_script_commands_watched(state, dir, &script(commands), None, Some(session)).unwrap();
}

#[test]
fn the_record_is_written_after_a_save_and_only_then() {
    let dir = SaveDir::new("write");
    let mut state = town();
    let mut session = TerminalSession::for_play(dir.path(), true);
    session.start(&state);
    play(&mut state, dir.path(), &mut session, &["d", "s"]);
    assert!(!dir.gem_file().exists(), "no save, no file");
    play(&mut state, dir.path(), &mut session, &["Q", "Y"]);
    assert!(dir.gem_file().exists(), "the save wrote the record");

    // A new run on the same save reads it back.
    let mut again = TerminalGemMap::for_save_dir(dir.path());
    assert_eq!(
        again.attach(&state),
        u5_automap::gem::AttachOutcome::Restored
    );
    assert_eq!(
        again.session().knowledge(),
        session.gem_map.as_ref().unwrap().session().knowledge()
    );
}

#[test]
fn an_older_save_restores_the_older_record() {
    let dir = SaveDir::new("older");
    let mut state = town();
    let mut session = TerminalSession::for_play(dir.path(), true);
    session.start(&state);
    play(&mut state, dir.path(), &mut session, &["Q", "Y"]);
    let older_record = fs::read(dir.gem_file()).unwrap();
    let older_save = fs::read(dir.path().join(SAVED_GAM_FILENAME)).unwrap();

    // Play on, learn something (a dungeon entered, its first cells), and
    // save again.
    let mut outside = world_state(open_world_grid(), 50, 50);
    outside.area = Area::World {
        plane: WorldPlane::Britannia,
    };
    session.before_command(&outside);
    session.after_command(&lit_dungeon(0, 1, 1));
    play(&mut state, dir.path(), &mut session, &["d", "Q", "Y"]);
    assert_ne!(fs::read(dir.gem_file()).unwrap(), older_record);

    // The older save comes back with its folder, as a slot carries it.
    fs::write(dir.path().join(SAVED_GAM_FILENAME), &older_save).unwrap();
    fs::write(dir.gem_file(), &older_record).unwrap();
    let mut reload = TerminalGemMap::for_save_dir(dir.path());
    assert_eq!(
        reload.attach(&state),
        u5_automap::gem::AttachOutcome::Restored
    );
    let store = u5_runtime::enhancements::SidecarStore::open(dir.path()).unwrap();
    let (older, _) = u5_automap::gem::attach(
        &store,
        &u5_runtime::enhancements::SaveStamp::of_image(&older_save),
    );
    let mut expected = older;
    for event in u5_automap::gem::attach_events(&state) {
        expected.apply(&event);
    }
    assert_eq!(reload.session().knowledge(), &expected);
}

#[test]
fn off_opens_no_store_and_leaves_an_old_record_alone() {
    let dir = SaveDir::new("off");
    // A record from an earlier run with the gem map on.
    let mut state = town();
    let mut on = TerminalSession::for_play(dir.path(), true);
    on.start(&state);
    play(&mut state, dir.path(), &mut on, &["Q", "Y"]);
    let file = dir.gem_file();
    fs::File::options()
        .write(true)
        .open(&file)
        .unwrap()
        .set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(2_000_000))
        .unwrap();
    let bytes = fs::read(&file).unwrap();
    let mtime = fs::metadata(&file).unwrap().modified().unwrap();

    let mut off = TerminalSession::for_play(dir.path(), false);
    assert!(off.is_empty(), "off keeps no watch and no record");
    off.start(&state);
    let mut state = town();
    play(
        &mut state,
        dir.path(),
        &mut off,
        &["d", "Q", "Y", "@gemmap"],
    );
    assert_eq!(fs::read(&file).unwrap(), bytes);
    assert_eq!(fs::metadata(&file).unwrap().modified().unwrap(), mtime);

    // With no record on disk, off makes no sidecar folder at all.
    let fresh = SaveDir::new("off-fresh");
    let mut state = town();
    let mut off = TerminalSession::for_play(fresh.path(), false);
    play(&mut state, fresh.path(), &mut off, &["Q", "Y"]);
    assert!(!fresh.path().join(SIDECAR_DIR).exists());
}

#[test]
fn saved_gam_is_the_same_on_and_off() {
    let run = |on: bool| {
        let dir = SaveDir::new(if on { "bytes-on" } else { "bytes-off" });
        let mut state = town();
        let mut session = TerminalSession::for_play(dir.path(), on);
        session.start(&state);
        play(
            &mut state,
            dir.path(),
            &mut session,
            &["d", "Q", "Y", "s", "QY"],
        );
        let bytes = fs::read(dir.path().join(SAVED_GAM_FILENAME)).unwrap();
        (bytes, state.turn, state.message.clone())
    };
    assert_eq!(run(true), run(false));
}

// ---------------------------------------------------------------------------
// The flag, the lock and equivalence.

fn resolved_gem(lock: LockState) -> u5_runtime::enhancements::ResolvedEnhancements {
    resolve_as_shipped(
        ResolveInput {
            stored: None,
            cli: &[CliSetting::Set {
                key: Toggle::GemMapEnhanced.key().to_string(),
                value: "true".to_string(),
            }],
            lock,
            caps: HostCapabilities::ALL,
        },
        &[GEM_G2],
    )
}

#[test]
fn the_flag_parses_for_the_play_and_route_modes_only() {
    let args = parse_cli_args(["--play", "--gem-map", "/g"]).unwrap();
    assert!(args.gem_map);
    let args = parse_cli_args(["--route-smoke", "--gem-map", "--parity-lock", "/g"]).unwrap();
    assert!(args.gem_map && args.parity_lock);
    assert!(args.enhancements.is_empty(), "route smoke resolves nothing");
    for refused in [
        vec!["--save-frame", "out.png", "--gem-map", "/g"],
        vec!["--equivalence-suite", "out", "--gem-map", "/g"],
    ] {
        assert!(
            parse_cli_args(refused.iter().copied()).is_err(),
            "{refused:?}"
        );
    }
    assert!(!parse_cli_args(["--play", "/g"]).unwrap().gem_map);
    assert!(cli_usage().contains("--gem-map"));
}

#[test]
fn the_terminal_feature_follows_the_toggle_and_the_lock() {
    assert!(
        terminal_features(&u5_runtime::enhancements::ResolvedEnhancements::classic()).is_empty()
    );
    let on = terminal_features(&resolved_gem(LockState::Off));
    assert_eq!(on.len(), 1);
    assert_eq!(on[0].toggle(), Toggle::GemMapEnhanced);
    assert_eq!(on[0].class(), EffectClass::Presentation);
    assert!(on[0].owns_token(GEM_MAP_TOKEN));
    assert!(terminal_features(&resolved_gem(LockState::Locked(LockReason::CliFlag))).is_empty());
}

#[test]
fn the_gem_map_is_equivalent_and_its_token_is_removed() {
    let state = town();
    let commands = script(&["d", "@gemmap", "s", "a", "@gemmap", "w"]);
    let mismatches = check_equivalence(
        &state,
        Path::new("."),
        &commands,
        terminal_features(&resolved_gem(LockState::Off)),
        None,
    );
    assert_eq!(mismatches, []);
}

/// Every step's trajectory digest of a route-smoke run.
#[derive(Default)]
struct Digests(Vec<u5_runtime::enhancements::TrajectoryDigest>);

impl RouteTraceObserver for Digests {
    fn after_command(&mut self, state: &PlayState, _index: usize, _command: &str) {
        self.0
            .push(u5_runtime::enhancements::trajectory_digest(state));
    }
}

/// The equivalence harness's check for a class (a) feature (design
/// 1.3.12), with the gem map's record kept: every route-smoke case passes
/// through the same states, frames and report with it on as with it off.
#[test]
fn every_route_case_is_the_same_with_the_gem_map() {
    let Some(dir) = assets() else {
        eprintln!("skipped: set U5_TEST_ASSET_DIR");
        return;
    };
    let atlas = load_tile_atlas(&dir, TileGraphicsDepth::Ega16).unwrap();
    for case in route_smoke_cases() {
        let mut off = Digests::default();
        let plain =
            run_route_smoke_case_traced(&dir, &atlas, &case, &mut NoScriptTokenHooks, &mut off)
                .map_err(|error| error.to_string());
        let mut on = Digests::default();
        let mut hooks = FeatureHooks::new(terminal_features(&resolved_gem(LockState::Off)));
        let watched = run_route_smoke_case_traced(&dir, &atlas, &case, &mut hooks, &mut on)
            .map_err(|error| error.to_string());
        assert_eq!(plain, watched, "{}", case.name);
        assert_eq!(off.0, on.0, "{}", case.name);
    }
}

// ---------------------------------------------------------------------------
// `--gem-map-png` and `--gem-map-canvas` (design 10.3.6, build plan M3-17).

fn synthetic_png(dir: &Path, canvas: (u32, u32)) -> GemPng {
    GemPng::terminal(
        dir,
        u5_automap::gem::GemCanvas {
            w: canvas.0,
            h: canvas.1,
        },
        dir,
        TileGraphicsDepth::Ega16,
    )
    .with_art(u5_automap::gem::MapArt::build(
        &u5_runtime::test_fixtures::synthetic_tile_atlas(TileGraphicsDepth::Ega16),
    ))
}

fn outdoors_with_gems(gems: u8) -> PlayState {
    let mut state = world_state(open_world_grid(), 100, 100);
    state.area = Area::World {
        plane: WorldPlane::Britannia,
    };
    state.gems = gems;
    state
}

fn png_session(png: GemPng) -> TerminalSession {
    TerminalSession {
        save_watch: None,
        gem_map: Some(TerminalGemMap::in_memory().with_png(png)),
    }
}

fn written(session: &TerminalSession) -> Vec<PathBuf> {
    session
        .gem_map
        .as_ref()
        .and_then(TerminalGemMap::png)
        .map(|png| png.written().to_vec())
        .unwrap_or_default()
}

/// One PNG per gem view open, at the canvas asked for: two gems used, two
/// pictures; the key that closes the view and the moves in between write
/// none. The game runs exactly as without the writer.
#[test]
fn one_png_is_written_per_gem_view_open_at_the_canvas() {
    let dir = SaveDir::new("png-per-open");
    let out = dir.path().join("shots");
    let commands = ["v", "s", "d", "v", "s", "d"];

    let mut plain = outdoors_with_gems(3);
    run_play_script_commands_watched(&mut plain, dir.path(), &script(&commands), None, None)
        .unwrap();

    let mut state = outdoors_with_gems(3);
    let mut session = png_session(synthetic_png(&out, (512, 240)));
    session.start(&state);
    for command in commands {
        play(&mut state, dir.path(), &mut session, &[command]);
    }
    assert_eq!(state.gems, 1, "two gems spent, as without the writer");
    assert_eq!(state.gems, plain.gems);
    assert_eq!(state.turn, plain.turn);
    assert_eq!(state.prng_state, plain.prng_state);
    assert_eq!(state.message, plain.message);

    let files = written(&session);
    assert_eq!(files.len(), 2, "{files:?}");
    for file in &files {
        let image = image::open(file).unwrap().to_rgba8();
        assert_eq!(image.dimensions(), (512, 240));
        assert!(
            image.pixels().all(|pixel| pixel.0[3] == 255),
            "an outdoor map is opaque"
        );
    }
    let names: Vec<String> = fs::read_dir(&out)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    assert_eq!(names.len(), 2);
}

/// A view that stays open across commands is one open: one picture.
#[test]
fn a_view_still_open_is_not_written_again() {
    let dir = SaveDir::new("png-held");
    let out = dir.path().join("shots");
    let mut png = synthetic_png(&out, (320, 200));
    let mut state = outdoors_with_gems(2);
    state.active_view_overlay = Some(u5_runtime::ViewOverlay {
        title: String::new(),
        text_map: String::new(),
        kind: u5_runtime::ViewOverlayKind::Surface,
        mode: u5_runtime::ViewOverlayMode::GemView,
    });
    let know = GemKnowledge::new();
    png.after(&state, &know);
    png.after(&state, &know);
    assert_eq!(png.written().len(), 1);
    // Closed, then opened again with the next gem: a second picture.
    let mut closed = state.clone();
    closed.active_view_overlay = None;
    png.after(&closed, &know);
    state.gems -= 1;
    png.after(&state, &know);
    assert_eq!(png.written().len(), 2);
    // Peer is not the gem view: nothing.
    let mut peer = state.clone();
    peer.gems -= 1;
    peer.active_view_overlay.as_mut().unwrap().mode = u5_runtime::ViewOverlayMode::PeerSpell;
    png.after(&peer, &know);
    assert_eq!(png.written().len(), 2);
}

/// The flags parse only together with `--gem-map` and a play run, and the
/// canvas only with the PNG folder, from 320x200 up.
#[test]
fn the_png_flags_need_the_gem_map_and_a_play_run() {
    let args = parse_cli_args([
        "--play-script",
        "v",
        "--gem-map",
        "--gem-map-png",
        "shots",
        "--gem-map-canvas",
        "512x240",
        "/g",
    ])
    .unwrap();
    assert_eq!(args.gem_map_png, Some(PathBuf::from("shots")));
    assert_eq!(args.gem_map_canvas, Some((512, 240)));
    let args = parse_cli_args(["--play", "--gem-map", "--gem-map-png", "shots", "/g"]).unwrap();
    assert_eq!(args.gem_map_canvas, None, "the classic canvas by default");
    for refused in [
        // No PNG without --gem-map.
        vec!["--play", "--gem-map-png", "shots", "/g"],
        // A canvas without the folder.
        vec!["--play", "--gem-map", "--gem-map-canvas", "512x240", "/g"],
        // Not a play run.
        vec!["--route-smoke", "--gem-map", "--gem-map-png", "shots", "/g"],
        vec!["--gem-map", "--gem-map-png", "shots", "/g"],
        // Below the classic canvas, or not WxH.
        vec![
            "--play",
            "--gem-map",
            "--gem-map-png",
            "s",
            "--gem-map-canvas",
            "300x200",
            "/g",
        ],
        vec![
            "--play",
            "--gem-map",
            "--gem-map-png",
            "s",
            "--gem-map-canvas",
            "512",
            "/g",
        ],
        vec![
            "--play",
            "--gem-map",
            "--gem-map-png",
            "s",
            "--gem-map-canvas",
            "9000x240",
            "/g",
        ],
    ] {
        assert!(
            parse_cli_args(refused.iter().copied()).is_err(),
            "{refused:?}"
        );
    }
    assert!(cli_usage().contains("--gem-map-png <DIR>"));
    assert!(cli_usage().contains("--gem-map-canvas <WxH>"));
}

/// The binary: with the parity lock, `--gem-map` is ignored, so no probe
/// runs and no PNG is written; with it, a gem used outdoors writes one.
/// Needs the player's files (`U5_TEST_ASSET_DIR`).
#[test]
fn the_binary_writes_pngs_only_with_the_gem_map_on() {
    let Some(assets) = assets() else {
        eprintln!("skipped: set U5_TEST_ASSET_DIR");
        return;
    };
    let scratch = SaveDir::new("png-binary");
    let run = |name: &str, lock: bool| {
        let out = scratch.path().join(name);
        let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_u5-engine"));
        command
            .env(
                RUNTIME_DIR_ENV,
                scratch.path().join(format!("runtime-{name}")),
            )
            .args([
                "--play-script",
                "@gemmap;v;s;@gemmap",
                "--scene",
                "BRITANNIA",
                "--at",
                "76,108",
                "--gem-map",
                "--gem-map-png",
            ])
            .arg(&out)
            .args(["--gem-map-canvas", "512x240"]);
        if lock {
            command.arg("--parity-lock");
        }
        let output = command.arg(&assets).output().unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        assert!(output.status.success(), "{stderr}");
        let files = fs::read_dir(&out)
            .map(|dir| dir.count())
            .unwrap_or_default();
        (stdout, stderr, files)
    };
    let (stdout, stderr, files) = run("locked", true);
    assert_eq!(files, 0, "no PNG under the lock");
    assert!(stderr.contains(PARITY_LOCK_LINE));
    assert!(
        !stdout.contains(u5_automap::gem::GEM_SUMMARY_PREFIX),
        "no record under the lock"
    );
    let (stdout, _, files) = run("on", false);
    assert!(stdout.contains(u5_automap::gem::GEM_SUMMARY_PREFIX));
    // A gem is spent only when the party has one.
    assert!(files <= 1, "{files}");
}

/// On the player's files (`U5_TEST_ASSET_DIR`): a gem used outdoors in a
/// play script writes one PNG through the real art (tiles and fonts), and
/// its pixels are the renderer's for the same view and record.
#[test]
fn a_real_gem_view_writes_the_renderers_picture() {
    use u5_automap::gem::{GemCanvas, GemImage, GemViewState, MapArt, render_gem_map};
    let Some(assets) = assets() else {
        eprintln!("skipped: set U5_TEST_ASSET_DIR");
        return;
    };
    let scratch = SaveDir::new("png-real");
    let out = scratch.path().join("shots");
    let canvas = GemCanvas { w: 512, h: 240 };
    let mut state = PlayState::load_scene(
        &assets,
        u5_runtime::PlayOptions {
            target: u5_runtime::PlayTarget::World(WorldPlane::Britannia),
            start: Some((76, 108)),
            clock: u5_runtime::GameClock::new(12, 6).unwrap(),
            ..u5_runtime::PlayOptions::default()
        },
    )
    .unwrap();
    state.gems = 2;
    let png = GemPng::terminal(&out, canvas, &assets, TileGraphicsDepth::Ega16);
    let mut session = png_session(png);
    session.start(&state);
    // The view opens on `v`; the picture is taken then, before `s`
    // closes it.
    play(&mut state, &assets, &mut session, &["v"]);
    let open = state.clone();
    play(&mut state, &assets, &mut session, &["s"]);
    assert_eq!(state.gems, 1);
    let files = written(&session);
    assert_eq!(files.len(), 1, "{files:?}");
    let picture = image::open(&files[0]).unwrap().to_rgba8();
    assert_eq!(picture.dimensions(), (512, 240));

    let atlas = load_tile_atlas(&assets, TileGraphicsDepth::Ega16).unwrap();
    let art = MapArt::build(&atlas).with_fonts(
        u5_runtime::load_ibm_ch_font(&assets).unwrap(),
        u5_runtime::load_runes_ch_font(&assets).unwrap(),
    );
    let know = session.gem_map.as_ref().unwrap().session().knowledge();
    // The terminal's settings: the catalogue's defaults, the map on.
    let t = u5_runtime::enhancements::GemMapToggles {
        enhanced: true,
        ..Default::default()
    };
    let view = GemViewState::open(&open, canvas, 100);
    let mut expected = GemImage::default();
    render_gem_map(&open, know, &art, &t, &view, &mut expected);
    assert!(
        picture.as_raw() == &expected.rgba,
        "the PNG is the renderer's picture"
    );
}

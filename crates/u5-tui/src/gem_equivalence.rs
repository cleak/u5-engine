//! The gem consumption cases of the equivalence suite (Phase B design 10.6,
//! "Gem consumption equivalence"; build plan M3-18).
//!
//! Using a gem must cost exactly what it costs in 1.0.1, whatever the
//! enhanced gem map does while the view is up. Five cases press `V`:
//! outdoors, in a town and in a dungeon with a gem, outdoors with none, and
//! in combat with one. Each runs from one start state with a configuration
//! on and again with every feature off (its `@` tokens removed, as
//! [`crate::equivalence`] does for the route cases), and the two runs must
//! agree after every command on:
//!
//! - the gem count;
//! - `active_view_overlay`: its kind, its mode, and its title and text map
//!   (by hash);
//! - `view_overlay_idle_polls`, `prng_state`, the clock and the message
//!   transcript (by hash);
//! - where the party is (area, cell and facing) and the whole
//!   [`TrajectoryDigest`];
//!
//! and after the run on the bytes of `SAVED.GAM`, the object files, the
//! world-progress file and the town NPC mutations after a scripted `Q`,
//! `Y` (the suite's [`SAVE_FILES`]).
//!
//! Each case also states what 1.0.1 does, and both runs must do it
//! ([`check_gem_expectation`]): a gem used outdoors, in a town or in a
//! dungeon is spent once and opens the gem view, and the next key (the
//! dismissal) closes it and is thrown away, moving nothing; a `V` with no
//! gem and a `V` in combat open nothing and spend nothing.
//!
//! The cases are not route-smoke cases: the route smoke and its digest
//! (`22254c83f2af767b`) are unchanged. `qa/enhancements/gem-cases.tsv` lists
//! them; a test keeps that file equal to [`gem_cases_tsv`].

use std::fs;
use std::io;
use std::path::Path;

use u5_runtime::enhancements::{TrajectoryDigest, fnv1a64, trajectory_digest};
use u5_runtime::{
    BRIT_OOL_FILENAME, DungeonScene, PlayOptions, PlayState, PlayTarget, Scene, UNDER_OOL_FILENAME,
    ViewOverlay, ViewOverlayMode, WorldPlane,
};

use crate::equivalence::{Mismatch, scripted_save};
use crate::play_loop::{ScriptTokenHooks, replay_play_script_commands_hooked};
use crate::route_smoke::{
    apply_route_smoke_case_setup, route_smoke_temp_dir, seed_route_smoke_save_files,
};

/// The committed list of the cases, beside the matrix.
pub const GEM_CASES_FILE: &str = "qa/enhancements/gem-cases.tsv";

/// Where a case starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GemPlace {
    /// Britannia, on foot.
    Outdoors,
    /// Lord British's castle, the route smoke's town.
    Town,
    /// The route smoke's dungeon, first level.
    Dungeon,
    /// Terrain combat entered from Britannia, the party's turn.
    Combat,
}

impl GemPlace {
    pub fn key(self) -> &'static str {
        match self {
            Self::Outdoors => "outdoors",
            Self::Town => "town",
            Self::Dungeon => "dungeon",
            Self::Combat => "combat",
        }
    }
}

/// What 1.0.1 does with the case's `V`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GemExpect {
    /// One gem is spent and the gem view opens; the next key closes it and
    /// is thrown away.
    Opens,
    /// Nothing opens and nothing is spent.
    Nothing,
}

impl GemExpect {
    pub fn key(self) -> &'static str {
        match self {
            Self::Opens => "opens-and-spends-one",
            Self::Nothing => "opens-nothing-spends-nothing",
        }
    }
}

/// One gem consumption case.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GemCase {
    pub name: &'static str,
    pub place: GemPlace,
    pub gems: u8,
    /// The play script. The command after `v` is the dismissal key.
    pub script: &'static [&'static str],
    pub expect: GemExpect,
}

/// The five cases of design 10.6. The dismissal key is a step (`s`, south),
/// so a key that reached the game instead of being thrown away would move
/// the party and be caught; a further step and a pass follow, so the turn
/// the gem owes and the world after it are compared too.
pub fn gem_consumption_cases() -> Vec<GemCase> {
    vec![
        GemCase {
            name: "gem-outdoors",
            place: GemPlace::Outdoors,
            gems: 2,
            script: &["v", "s", "a", "pass"],
            expect: GemExpect::Opens,
        },
        GemCase {
            name: "gem-town",
            place: GemPlace::Town,
            gems: 2,
            script: &["v", "s", "a", "pass"],
            expect: GemExpect::Opens,
        },
        GemCase {
            name: "gem-dungeon",
            place: GemPlace::Dungeon,
            gems: 2,
            script: &["v", "s", "pass"],
            expect: GemExpect::Opens,
        },
        GemCase {
            name: "gem-none",
            place: GemPlace::Outdoors,
            gems: 0,
            script: &["v", "s", "a", "pass"],
            expect: GemExpect::Nothing,
        },
        GemCase {
            name: "gem-combat",
            place: GemPlace::Combat,
            gems: 2,
            script: &["v", "pass"],
            expect: GemExpect::Nothing,
        },
    ]
}

/// The cases as `qa/enhancements/gem-cases.tsv`: names, places, gem
/// counts, scripts and what 1.0.1 does. No game data.
pub fn gem_cases_tsv(cases: &[GemCase]) -> String {
    let mut out = String::from(
        "# Ultima V gem consumption equivalence cases (Phase B design 10.6).\n\
         # Generated from u5_tui::gem_equivalence; a test keeps this file equal\n\
         # to it (cargo test -p u5-tui --test gem_equivalence;\n\
         # U5_BLESS_GEM_CASES=1 rewrites it). The equivalence suite runs every\n\
         # case under Classic and each matrix row; none is a route-smoke case.\n\
         # name\tplace\tgems\tscript\texpect\n",
    );
    for case in cases {
        out.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\n",
            case.name,
            case.place.key(),
            case.gems,
            case.script.join(" "),
            case.expect.key()
        ));
    }
    out
}

/// The case's start state from the game in `game_dir`, with the route
/// smoke's fixed random seed.
pub fn gem_case_state(game_dir: &Path, case: &GemCase) -> io::Result<PlayState> {
    let target = match case.place {
        GemPlace::Outdoors | GemPlace::Combat => PlayTarget::World(WorldPlane::Britannia),
        GemPlace::Town => PlayTarget::Town(Scene::new(0x11).expect("the castle scene is valid")),
        GemPlace::Dungeon => {
            PlayTarget::Dungeon(DungeonScene::new(0x21).expect("the dungeon scene is valid"))
        }
    };
    let options = PlayOptions {
        target,
        floor: 0,
        gems: case.gems,
        ..PlayOptions::default()
    };
    let mut state = PlayState::load_scene(game_dir, options)?;
    // The route smoke's seed (and, for combat, its terrain-combat entry).
    let setup = match case.place {
        GemPlace::Combat => "terrain-combat-party-entry",
        _ => case.name,
    };
    apply_route_smoke_case_setup(&mut state, setup, game_dir)?;
    state.gems = case.gems;
    Ok(state)
}

/// What one command left, as the gem cases compare it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GemBeat {
    pub index: usize,
    pub command: String,
    pub gems: u8,
    /// `kind mode title-hash map-hash`, or `none`.
    pub overlay: String,
    pub gem_view: bool,
    pub idle_polls: u32,
    pub prng: u16,
    pub clock: String,
    pub transcript: u64,
    /// Area, cell and facing.
    pub position: String,
    pub digest: TrajectoryDigest,
}

impl GemBeat {
    fn of(state: &PlayState, index: usize, command: &str) -> Self {
        Self {
            index,
            command: command.to_string(),
            gems: state.gems,
            overlay: overlay_line(state.active_view_overlay.as_ref()),
            gem_view: is_gem_view(state),
            idle_polls: state.view_overlay_idle_polls,
            prng: state.prng_state,
            clock: format!("{:?}", state.clock),
            transcript: trajectory_digest(state).transcript,
            position: position_line(state),
            digest: trajectory_digest(state),
        }
    }
}

fn overlay_line(overlay: Option<&ViewOverlay>) -> String {
    overlay.map_or_else(
        || "none".to_string(),
        |o| {
            format!(
                "{:?} {:?} title {:016x} map {:016x}",
                o.kind,
                o.mode,
                fnv1a64(o.title.as_bytes()),
                fnv1a64(o.text_map.as_bytes())
            )
        },
    )
}

fn is_gem_view(state: &PlayState) -> bool {
    state
        .active_view_overlay
        .as_ref()
        .is_some_and(|o| o.mode == ViewOverlayMode::GemView)
}

fn position_line(state: &PlayState) -> String {
    format!(
        "{} at ({}, {}) facing {}",
        state.current_area_label(),
        state.player.x,
        state.player.y,
        state.player.facing.name()
    )
}

/// One run of a case.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GemRun {
    /// The start state's beat, then one per command run.
    pub beats: Vec<GemBeat>,
    pub saves: Vec<(&'static str, Option<u64>)>,
    pub save_error: Option<String>,
    pub error: Option<String>,
}

/// Runs `commands` from `state` with `hooks`, then saves with `Q`, `Y` into
/// `save_dir` when one is given.
pub fn run_gem_commands(
    mut state: PlayState,
    game_dir: &Path,
    commands: &[String],
    hooks: &mut dyn ScriptTokenHooks,
    save_dir: Option<&Path>,
) -> GemRun {
    let mut run = GemRun {
        beats: vec![GemBeat::of(&state, usize::MAX, "start")],
        ..GemRun::default()
    };
    let result = replay_play_script_commands_hooked(
        &mut state,
        game_dir,
        commands,
        hooks,
        |state, index, command| {
            run.beats.push(GemBeat::of(state, index, command));
            Ok(())
        },
    );
    if let Err(error) = result {
        run.error = Some(error.to_string());
        return run;
    }
    if let Some(dir) = save_dir {
        match scripted_save(&mut state, dir) {
            Ok(saves) => run.saves = saves,
            Err(error) => run.save_error = Some(error.to_string()),
        }
    }
    run
}

/// Runs `case` from the game in `game_dir` with `hooks`. The object files
/// the run writes into the game folder are put back afterwards, and the
/// scripted save goes into a scratch copy of the seed save.
pub fn run_gem_case(
    game_dir: &Path,
    case: &GemCase,
    hooks: &mut dyn ScriptTokenHooks,
    label: &str,
) -> io::Result<GemRun> {
    let brit = fs::read(game_dir.join(BRIT_OOL_FILENAME))?;
    let under = fs::read(game_dir.join(UNDER_OOL_FILENAME))?;
    let commands: Vec<String> = case.script.iter().map(|c| (*c).to_string()).collect();
    let run = match gem_case_state(game_dir, case) {
        Ok(state) => {
            let dir = route_smoke_temp_dir(case.name, label)?;
            let run = match seed_route_smoke_save_files(game_dir, &dir) {
                Ok(()) => run_gem_commands(state, game_dir, &commands, hooks, Some(&dir)),
                Err(error) => GemRun {
                    error: Some(format!("seed save: {error}")),
                    ..GemRun::default()
                },
            };
            let _ = fs::remove_dir_all(&dir);
            run
        }
        Err(error) => GemRun {
            error: Some(format!("start state: {error}")),
            ..GemRun::default()
        },
    };
    fs::write(game_dir.join(BRIT_OOL_FILENAME), &brit)?;
    fs::write(game_dir.join(UNDER_OOL_FILENAME), &under)?;
    Ok(run)
}

/// Compares an on run with its off run. Empty means equivalent. Each
/// compared part reports the first command it parts at.
pub fn compare_gem_runs(off: &GemRun, on: &GemRun) -> Vec<Mismatch> {
    let mut out = Vec::new();
    let mut differ = |what: String, off: String, on: String| out.push(Mismatch { what, off, on });
    if off.error != on.error {
        differ(
            "error".into(),
            format!("{:?}", off.error),
            format!("{:?}", on.error),
        );
    }
    if off.beats.len() != on.beats.len() {
        differ(
            "commands run".into(),
            off.beats.len().to_string(),
            on.beats.len().to_string(),
        );
    }
    type Part = fn(&GemBeat) -> String;
    let parts: [(&str, Part); 9] = [
        ("gem count", |b| b.gems.to_string()),
        ("view overlay", |b| b.overlay.clone()),
        ("idle polls", |b| b.idle_polls.to_string()),
        ("prng", |b| format!("{:04x}", b.prng)),
        ("clock", |b| b.clock.clone()),
        ("transcript", |b| format!("{:016x}", b.transcript)),
        ("position", |b| b.position.clone()),
        ("command", |b| b.command.clone()),
        ("digest", |b| b.digest.to_string()),
    ];
    for (what, part) in parts {
        if let Some((a, b)) = off
            .beats
            .iter()
            .zip(&on.beats)
            .find(|(a, b)| part(a) != part(b))
        {
            let at = if a.index == usize::MAX {
                "start".to_string()
            } else {
                format!("step {} `{}`", a.index + 1, a.command)
            };
            differ(format!("{at} {what}"), part(a), part(b));
        }
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
    out
}

/// Whether a run does what 1.0.1 does with the case's `V`; the reason when
/// it does not.
pub fn check_gem_expectation(case: &GemCase, run: &GemRun) -> Result<(), String> {
    if let Some(error) = &run.error {
        return Err(format!("the run failed: {error}"));
    }
    let start = run.beats.first().ok_or("no start beat")?;
    if start.gems != case.gems || start.gem_view {
        return Err("the start state is not the case's".into());
    }
    let v = run
        .beats
        .iter()
        .position(|b| b.index == 0)
        .ok_or("the `v` never ran")?;
    match case.expect {
        GemExpect::Opens => {
            let opened = &run.beats[v];
            if !opened.gem_view {
                return Err(format!("`v` opened no gem view ({})", opened.overlay));
            }
            if opened.gems + 1 != case.gems {
                return Err(format!(
                    "`v` left {} gem(s) of {}, not one fewer",
                    opened.gems, case.gems
                ));
            }
            let dismissed = run.beats.get(v + 1).ok_or("no dismissal key ran")?;
            if dismissed.gem_view || dismissed.overlay != "none" {
                return Err(format!(
                    "the dismissal key `{}` left the view up",
                    dismissed.command
                ));
            }
            if dismissed.position != opened.position {
                return Err(format!(
                    "the dismissal key `{}` reached the game: {} -> {}",
                    dismissed.command, opened.position, dismissed.position
                ));
            }
            if let Some(later) = run.beats[v..].iter().find(|b| b.gems != opened.gems) {
                return Err(format!("a gem was spent again at `{}`", later.command));
            }
        }
        GemExpect::Nothing => {
            if let Some(beat) = run.beats.iter().find(|b| b.gem_view) {
                return Err(format!("a gem view opened at `{}`", beat.command));
            }
            if let Some(beat) = run.beats.iter().find(|b| b.gems != case.gems) {
                return Err(format!("a gem was spent at `{}`", beat.command));
            }
        }
    }
    Ok(())
}

//! The dispatcher cross-check for `PlayState::save_command_available`
//! (Phase B design 3.3.4, 3.6; plan M4-10).
//!
//! After every command of every route-smoke case, a clone of the state is
//! fed `Q` through the dispatcher, and the save prompt must open exactly
//! when the predicate says it would:
//!
//! - where the predicate is true, `Q` opens `Save game?`;
//! - at the idle exploration prompt, where the menu is open, a false
//!   predicate means `Q` does something else. The one exception is a drunk
//!   party in a town, whose `Q` takes a coin toss first; the predicate
//!   refuses it, and the toss is not counted here;
//! - away from the idle prompt the predicate is false by its definition
//!   (the menu is never open there), so a `Q` that still reaches the prompt
//!   there is counted, not failed.
//!
//! It is a test of its own: the route smoke and its manifest digest are not
//! touched, and the clones run in a scratch copy of the player's files so
//! no case sees another's writes. It needs `U5_TEST_ASSET_DIR`.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use u5_runtime::test_fixtures::configured_original_asset_dir;
use u5_runtime::z_stats::YesNoPromptKind;
use u5_runtime::{Area, PlayState, TileGraphicsDepth, handle_play_key_input, load_tile_atlas};
use u5_tui::*;

/// A writable copy of the player's files, removed on drop.
struct Copy(PathBuf);

impl Copy {
    fn of(dir: &Path, label: &str) -> Self {
        let unique = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let copy = std::env::temp_dir().join(format!(
            "u5-tui-save-predicate-{label}-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&copy).unwrap();
        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_file() {
                fs::copy(entry.path(), copy.join(entry.file_name())).unwrap();
            }
        }
        Self(copy)
    }
}

impl Drop for Copy {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn assets() -> Option<PathBuf> {
    let dir = configured_original_asset_dir()?;
    dir.join("CASTLE.DAT").exists().then_some(dir)
}

fn save_prompt_open(state: &PlayState) -> bool {
    state.active_yes_no_prompt.map(|session| session.kind) == Some(YesNoPromptKind::SaveGame)
}

/// Whether `Q` typed into a clone of `state` opens the save prompt.
fn q_opens_save(state: &PlayState, dir: &Path) -> bool {
    if save_prompt_open(state) {
        return false;
    }
    let mut clone = state.clone();
    let _ = handle_play_key_input(&mut clone, 'Q', "", dir);
    save_prompt_open(&clone)
}

#[derive(Default)]
struct Tally {
    states: usize,
    available: usize,
    idle_refused: usize,
    drunk: usize,
    conservative: usize,
    /// Why each counted non-idle state was not idle.
    conservative_blockers: std::collections::BTreeMap<String, usize>,
    failures: Vec<String>,
}

struct CrossCheck<'a> {
    case: &'static str,
    clones: &'a Path,
    tally: &'a mut Tally,
}

impl RouteTraceObserver for CrossCheck<'_> {
    fn after_command(&mut self, state: &PlayState, index: usize, command: &str) {
        let tally = &mut *self.tally;
        tally.states += 1;
        let predicate = state.save_command_available();
        let opens = q_opens_save(state, self.clones);
        let idle = state.at_idle_command_prompt();
        let drunk = matches!(state.area, Area::Town { .. }) && state.town_drunkenness_counter != 0;
        let at = || format!("{} step {} `{command}`", self.case, index + 1);
        if predicate {
            tally.available += 1;
            if !opens {
                tally.failures.push(format!(
                    "{}: predicate true, Q did not open the prompt",
                    at()
                ));
            }
        } else if idle && drunk {
            tally.drunk += 1;
        } else if idle {
            tally.idle_refused += 1;
            if opens {
                tally.failures.push(format!(
                    "{}: predicate false at the idle prompt, Q opened the prompt",
                    at()
                ));
            }
        } else if opens {
            tally.conservative += 1;
            let blocker = match state.idle_blocker() {
                Some(blocker) => format!("{blocker:?}"),
                None => "Combat".to_string(),
            };
            *tally.conservative_blockers.entry(blocker).or_default() += 1;
        }
    }
}

#[test]
fn the_save_predicate_agrees_with_the_dispatcher_on_every_route_state() {
    let Some(dir) = assets() else {
        eprintln!("skipped: set U5_TEST_ASSET_DIR");
        return;
    };
    let routes = Copy::of(&dir, "routes");
    let clones = Copy::of(&dir, "clones");
    let atlas = load_tile_atlas(&routes.0, TileGraphicsDepth::Ega16).unwrap();
    let mut tally = Tally::default();
    for case in route_smoke_cases() {
        let mut check = CrossCheck {
            case: case.name,
            clones: &clones.0,
            tally: &mut tally,
        };
        run_route_smoke_case_traced(
            &routes.0,
            &atlas,
            &case,
            &mut NoScriptTokenHooks,
            &mut check,
        )
        .unwrap_or_else(|error| panic!("route case {}: {error}", case.name));
    }
    eprintln!(
        "save predicate: {} states, {} save points, {} refused at the idle prompt, \
         {} drunk, {} non-idle states where Q still reaches the prompt {:?}",
        tally.states,
        tally.available,
        tally.idle_refused,
        tally.drunk,
        tally.conservative,
        tally.conservative_blockers
    );
    assert!(tally.available > 100, "the routes reach many save points");
    assert!(
        tally.failures.is_empty(),
        "{} disagreement(s):\n{}",
        tally.failures.len(),
        tally.failures.join("\n")
    );
}

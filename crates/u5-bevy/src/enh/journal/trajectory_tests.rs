//! T1, chapter 8's trajectory test (design 8.6; build plan M6-08), on the
//! foundation harness ([`crate::enh::equivalence`]).
//!
//! One scripted conversation, typed through the shell's own keyboard path,
//! runs three ways from one start state:
//!
//! - everything off (the 1.0.1 shell);
//! - every chapter 8 key on, `clue_highlight = new`, `tracker_evidence =
//!   npc`, `clue_misses` (forced, as its phase waits for D8.14), the
//!   chapter 5 page and panel at tier 4;
//! - the Recommended preset.
//!
//! After every frame the two "on" runs must equal the off run on the
//! trajectory digest, the original frame and the key queue, and on the
//! fields 8.6 names one by one: `prng_state`, the clock, the transcript,
//! `talk_branch_flags` and `conversation_signal_flags`. A scripted `Q`,
//! `Y` from the state mid-route and at the end must write the same save
//! bytes. `clue_text = kept` is not in this build (D8.1), so it is not run.
//!
//! The person and every word are invented; the `.TLK` file is written in
//! the `formats/tlk.md` layout into a scratch game folder.

use std::fs;
use std::path::{Path, PathBuf};

use bevy::prelude::KeyCode;
use u5_runtime::enhancements::{
    CLUES_P1D, CliSetting, HostCapabilities, LockState, Preset, ResolveInput, ResolvedEnhancements,
    SHIPPED_PHASES, resolve, resolve_as_shipped,
};
use u5_runtime::test_fixtures::{
    debug_game_dir, fix_host_clock_for_tests, open_grid, saved_game_seed_bytes, test_state,
    unique_scratch_dir, write_empty_ool_mirrors,
};
use u5_runtime::{
    BRIT_OOL_FILENAME, Direction, NpcSlot, PlayState, SAVED_GAM_FILENAME, SAVED_OOL_FILENAME,
    UNDER_OOL_FILENAME, handle_play_key_input,
};

use super::clues::ClueLogResource;
use crate::VisualLaunchConfig;
use crate::enh::equivalence::{Scene, Shell, ShellRun, Timeline, compare_shell_runs};

const DIALOG_ID: u8 = 0x10;
const SAVE_FILES: [&str; 4] = [
    SAVED_GAM_FILENAME,
    SAVED_OOL_FILENAME,
    BRIT_OOL_FILENAME,
    UNDER_OOL_FILENAME,
];

/// Bit 7 set, as the file stores text.
fn text(s: &str) -> Vec<u8> {
    s.bytes().map(|b| b | 0x80).collect()
}

fn record(parts: &[&[u8]]) -> Vec<u8> {
    parts.concat()
}

/// An invented person: the five leading records, pairs covering a plain
/// answer, a held page, a gold demand the party cannot meet, an ASK-WHO,
/// a GOTO into a block with a scoped prompt, a standing test with a
/// labelled branch and a runic span; then the two blocks.
fn fields() -> Vec<Vec<u8>> {
    let mut fields = vec![
        text("Quorvel"),
        text("a dusty quillwright"),
        text("Hail, wanderer."),
        text("I mend quills."),
        text("Fare well."),
    ];
    let pairs: Vec<(&str, Vec<u8>)> = vec![
        ("gran", text("Old one.")),
        ("mill", record(&[&text("Wheels "), &[0x83], &text("turn.")])),
        (
            "toll",
            record(&[&text("Pay "), &[0x85], &text("900"), &text(" now.")]),
        ),
        ("who", record(&[&text("Thy name? "), &[0x88]])),
        ("tree", record(&[&text("Climb? "), &[0x91]])),
        ("oxen", record(&[&[0xFE, 50, 0x92], &text("Begone.")])),
        (
            "rune",
            record(&[&text("See "), &[0x8E], &text("abc"), &[0x8E]]),
        ),
    ];
    for (keyword, response) in pairs {
        fields.push(text(keyword));
        fields.push(response);
    }
    // 0x91: a scoped yes/no prompt.
    let mut tree = vec![0x90, 0x91];
    tree.extend(text("Yes or no?"));
    fields.extend([
        tree,
        text("y"),
        text("Up thou goest."),
        text("n"),
        text("Stay then."),
    ]);
    // 0x92: the branch the standing test jumps to.
    let mut oxen = vec![0x90, 0x92];
    oxen.extend(text("Welcome."));
    oxen.push(0x82);
    fields.push(oxen);
    fields
}

/// A one-blob `.TLK`: the entry count, one (id, offset) row, the blob.
fn tlk_file() -> Vec<u8> {
    let mut file = vec![1, 0, DIALOG_ID, 0, 6, 0];
    for field in fields() {
        file.extend(field);
        file.push(0);
    }
    file
}

/// A scratch game folder and save seed, removed when dropped.
struct Folders {
    game: PathBuf,
    seed: PathBuf,
    sidecars: Vec<PathBuf>,
}

impl Folders {
    fn new() -> Self {
        let game = debug_game_dir();
        let scene = u5_runtime::npc_tlk_filename(0x11).unwrap();
        fs::write(game.join(scene), tlk_file()).unwrap();
        let seed = unique_scratch_dir("clue-trajectory-seed");
        fs::create_dir_all(&seed).unwrap();
        fs::write(
            seed.join(SAVED_GAM_FILENAME),
            saved_game_seed_bytes(0x01, 0, 5, 5),
        )
        .unwrap();
        write_empty_ool_mirrors(&seed);
        Self {
            game,
            seed,
            sidecars: Vec::new(),
        }
    }

    /// A save folder of its own for one run's sidecar.
    fn sidecar_dir(&mut self) -> PathBuf {
        let dir = unique_scratch_dir("clue-trajectory-saves");
        fs::create_dir_all(&dir).unwrap();
        self.sidecars.push(dir.clone());
        dir
    }
}

impl Drop for Folders {
    fn drop(&mut self) {
        for dir in [&self.game, &self.seed].into_iter().chain(&self.sidecars) {
            let _ = fs::remove_dir_all(dir);
        }
    }
}

/// The town with the person east of the party, who has ten gold.
fn start() -> PlayState {
    let mut state = test_state(open_grid(), 5, 5);
    state.gold = 10;
    state.player.facing = Direction::East;
    state.load_scheduled_npcs(&[
        NpcSlot {
            slot: 0,
            type_byte: 0,
            dialog_id: 0,
            schedule: [0; 16],
            name: None,
        },
        NpcSlot {
            slot: 1,
            type_byte: 1,
            dialog_id: DIALOG_ID,
            schedule: [0, 0, 0, 6, 6, 6, 5, 5, 5, 0, 0, 0, 0, 8, 16, 20],
            name: None,
        },
    ]);
    state
}

fn letter(c: char) -> KeyCode {
    use KeyCode::*;
    const KEYS: [KeyCode; 26] = [
        KeyA, KeyB, KeyC, KeyD, KeyE, KeyF, KeyG, KeyH, KeyI, KeyJ, KeyK, KeyL, KeyM, KeyN, KeyO,
        KeyP, KeyQ, KeyR, KeyS, KeyT, KeyU, KeyV, KeyW, KeyX, KeyY, KeyZ,
    ];
    assert!(c.is_ascii_lowercase(), "no key for {c:?}");
    KEYS[usize::from(c as u8 - b'a')]
}

/// Frames between taps, and after each Enter.
const KEY_GAP: usize = 4;
const LINE_GAP: usize = 30;

/// The route, every key the player's: Talk east, then NAME, JOB, a
/// keyword, a held page (Space), a gold demand refused, ASK-WHO answered
/// with the Avatar's name (which sets the acquaintance bit),
/// a GOTO and its scoped answer, a standing branch, a no-match, a runic
/// answer, a repeat, and an empty line to leave. Returns the timeline and
/// the frame after the JOB line, where the mid-route save is taken.
fn route() -> (Timeline, usize) {
    let mut taps: Vec<(usize, KeyCode)> = Vec::new();
    let mut frame = 5;
    let mut tap = |frame: &mut usize, key: KeyCode, gap: usize| {
        taps.push((*frame, key));
        *frame += gap;
    };
    tap(&mut frame, KeyCode::KeyT, KEY_GAP);
    tap(&mut frame, KeyCode::ArrowRight, LINE_GAP);
    let mut mid = 0;
    for line in [
        "name", "job", "gran", "mill", " ", "toll", "who", "avatar", "tree", "y", "oxen", "zzxq",
        "rune", "gran", "",
    ] {
        if line == " " {
            tap(&mut frame, KeyCode::Space, LINE_GAP);
            continue;
        }
        for c in line.chars() {
            tap(&mut frame, letter(c), KEY_GAP);
        }
        tap(&mut frame, KeyCode::Enter, LINE_GAP);
        if line == "job" {
            mid = frame;
        }
    }
    // A short walk after the conversation, so the world ticks on.
    for key in [KeyCode::ArrowUp, KeyCode::ArrowLeft, KeyCode::Space] {
        tap(&mut frame, key, LINE_GAP);
    }
    let mut timeline = Timeline::new(frame + 20);
    for (at, key) in taps {
        timeline.game_tap(at, key);
    }
    (timeline, mid)
}

/// `Q`, `Y` from `state` into a copy of the seed save: each file's bytes.
fn scripted_save(mut state: PlayState, seed: &Path) -> Vec<(&'static str, Option<Vec<u8>>)> {
    let dir = unique_scratch_dir("clue-trajectory-save");
    fs::create_dir_all(&dir).unwrap();
    for entry in fs::read_dir(seed).unwrap() {
        let entry = entry.unwrap();
        fs::copy(entry.path(), dir.join(entry.file_name())).unwrap();
    }
    handle_play_key_input(&mut state, 'Q', "", &dir).unwrap();
    handle_play_key_input(&mut state, 'Y', "", &dir).unwrap();
    let saves = SAVE_FILES
        .iter()
        .map(|name| (*name, fs::read(dir.join(name)).ok()))
        .collect();
    let _ = fs::remove_dir_all(&dir);
    saves
}

/// The fields 8.6 names, per frame.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Named {
    prng: u16,
    clock: String,
    transcript: u64,
    branch_flags: Vec<(u8, u32)>,
    signal_flags: Vec<u8>,
}

fn named(state: &PlayState) -> Named {
    let mut branch_flags: Vec<(u8, u32)> = state
        .talk_branch_flags
        .iter()
        .map(|(scene, bits)| (*scene, *bits))
        .collect();
    branch_flags.sort_unstable();
    Named {
        prng: state.prng_state,
        clock: format!("{:?}", state.clock),
        transcript: u5_runtime::enhancements::trajectory_digest(state).transcript,
        branch_flags,
        signal_flags: state.conversation_signal_flags.to_vec(),
    }
}

struct Trace {
    run: ShellRun,
    named: Vec<Named>,
    saves: Vec<Vec<(&'static str, Option<Vec<u8>>)>>,
    /// Entries in the clue log at the end, if the log was on.
    logged: Option<usize>,
    /// Whether a conversation was open at some frame.
    talked: bool,
}

fn run(launch: &VisualLaunchConfig, folders: &Folders, timeline: &Timeline, mid: usize) -> Trace {
    let scene = Scene {
        game_dir: folders.game.clone(),
        ..Scene::synthetic()
    };
    let mut shell = Shell::new(start(), &scene, launch, |_| {});
    let mut frames = Vec::with_capacity(timeline.len());
    let mut fields = Vec::with_capacity(timeline.len());
    let mut saves = Vec::new();
    let mut talked = false;
    for index in 0..timeline.len() {
        frames.push(shell.step(timeline.frame(index)));
        let state = &shell.visual().state;
        fields.push(named(state));
        talked |= state.active_conversation.is_some();
        if index == mid {
            saves.push(scripted_save(state.clone(), &folders.seed));
        }
    }
    saves.push(scripted_save(shell.visual().state.clone(), &folders.seed));
    let logged = shell
        .app
        .world()
        .get_resource::<ClueLogResource>()
        .map(|log| log.log().entries().len());
    Trace {
        run: ShellRun {
            frames,
            fed: Vec::new(),
        },
        named: fields,
        saves,
        logged,
        talked,
    }
}

fn launch_with(enhancements: ResolvedEnhancements, save_dir: PathBuf) -> VisualLaunchConfig {
    VisualLaunchConfig {
        enhancements,
        save_dir: Some(save_dir),
        ..VisualLaunchConfig::classic()
    }
}

fn input(cli: &[CliSetting]) -> ResolveInput<'_> {
    ResolveInput {
        stored: None,
        cli,
        lock: LockState::Off,
        caps: HostCapabilities::ALL,
    }
}

/// Every chapter 8 key on, with its spoiler values, `clue_misses` forced,
/// and chapter 5's page and panel at tier 4.
fn everything_on() -> ResolvedEnhancements {
    let on = |key: &str| CliSetting::Enable(key.into());
    let set = |key: &str, value: &str| CliSetting::Set {
        key: key.into(),
        value: value.into(),
    };
    let cli = [
        on("journal.clues"),
        set("journal.clue_text", "live"),
        set("journal.clue_highlight", "new"),
        set("journal.clue_highlight_style", "tint"),
        on("journal.clue_scrollback"),
        on("journal.clue_toast"),
        on("journal.notes"),
        on("journal.clue_misses"),
        on("journal.quest_tracker"),
        set("journal.tracker_evidence", "npc"),
        on("journal.page"),
        on("journal.side_panel"),
        set("journal.spoiler_tier", "4"),
    ];
    let shipped: Vec<_> = SHIPPED_PHASES.iter().copied().chain([CLUES_P1D]).collect();
    let resolved = resolve_as_shipped(input(&cli), &shipped);
    assert!(resolved.notes.is_empty(), "{:?}", resolved.notes);
    let clues = resolved.journal().clues;
    assert!(clues.clues && clues.misses && clues.spoiler_risk());
    resolved
}

fn recommended() -> ResolvedEnhancements {
    let resolved = resolve(input(&[CliSetting::Preset(Preset::Recommended)]));
    let clues = resolved.journal().clues;
    assert!(clues.clues && clues.quest_tracker && clues.observer_on());
    resolved
}

fn compare(off: &Trace, on: &Trace, label: &str) -> Vec<String> {
    let mut lines: Vec<String> = compare_shell_runs(&off.run, &on.run)
        .iter()
        .map(|mismatch| format!("{label}: {mismatch}"))
        .collect();
    if let Some((frame, (a, b))) = off
        .named
        .iter()
        .zip(&on.named)
        .enumerate()
        .find(|(_, (a, b))| a != b)
    {
        lines.push(format!("{label}: frame {frame}: off {a:?} / on {b:?}"));
    }
    for (index, (a, b)) in off.saves.iter().zip(&on.saves).enumerate() {
        for ((name, x), (_, y)) in a.iter().zip(b) {
            if x != y {
                lines.push(format!("{label}: save {index} {name} differs"));
            }
        }
    }
    lines
}

#[test]
fn t1_the_conversation_route_is_equal_three_ways() {
    // The stranger opener re-seeds from the host's time of day.
    fix_host_clock_for_tests();
    let mut folders = Folders::new();
    let (timeline, mid) = route();
    let off_dir = folders.sidecar_dir();
    let off = run(
        &launch_with(ResolvedEnhancements::classic(), off_dir),
        &folders,
        &timeline,
        mid,
    );
    // The route is not vacuous: the person talked, the game moved, the
    // saves wrote, and the party was introduced.
    assert!(off.talked, "the route opens the conversation");
    assert!(off.logged.is_none());
    assert_eq!(off.saves.len(), 2);
    assert!(off.saves.iter().all(|save| save[0].1.is_some()));
    let first = &off.named[0];
    assert!(
        off.named
            .iter()
            .any(|fields| fields.transcript != first.transcript)
    );
    assert!(
        off.named
            .last()
            .unwrap()
            .branch_flags
            .iter()
            .any(|(_, bits)| *bits != 0),
        "NAME sets the acquaintance bit"
    );

    let mut mismatches = Vec::new();
    for (label, enhancements) in [
        ("every key on", everything_on()),
        ("Recommended", recommended()),
    ] {
        let dir = folders.sidecar_dir();
        let on = run(&launch_with(enhancements, dir), &folders, &timeline, mid);
        assert!(
            on.logged.is_some_and(|entries| entries >= 4),
            "{label}: the log captured the route ({:?})",
            on.logged
        );
        mismatches.extend(compare(&off, &on, label));
    }
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

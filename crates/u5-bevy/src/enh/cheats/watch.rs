//! The change watch (Phase B design 7.2.8, 7.3.8; build plan M8-05).
//!
//! `watch_record` runs in `EnhSet::Observe` under its own run condition,
//! `toggle_on(StateWatch)`, while the [`WatchLog`] and the `VisualState`
//! exist. It reads the game through `Res<VisualState>` only: each frame it
//! takes the runtime's named-field snapshot, diffs it with the last one and
//! pushes the changes into a ring of [`WATCH_CAPACITY`] entries. It never
//! writes the game, so the watch is class (a): a run with it on is the run
//! with it off.
//!
//! The Watch face of the Cheats page shows the newest entries first. Its
//! keys pause and resume recording, clear the log, check the groups (the
//! Engine group, the random state and the turn counter, starts unchecked
//! and is only recorded while checked) and `[S]` save the shown log as
//! plain text to the folder the host gave ([`WatchFolder`]), never to the
//! game, save or runtime directory. The file holds our field labels and
//! the values, nothing copied from game text.

use std::collections::VecDeque;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use bevy::prelude::*;
use u5_runtime::cheats::{WatchField, WatchGroup, WatchValue, watch_diff, watch_snapshot};

use super::edit::{EditRun, Ink};
use crate::VisualState;

// ---------------------------------------------------------------------------
// Engine text (7.2.8).

pub const WATCH_TITLE: &str = "Watch";
pub const WATCH_TITLE_PAUSED: &str = "Watch: paused";
pub const WATCH_FOOTER: &str = "Esc: back";
/// The main page's entry.
pub const WATCH_ENTRY: &str = "[W] Watch";
pub const WATCH_HELP: &str = "Space pause  C clear  S save";
pub const WATCH_HEADS: [(u32, &str); 3] = [(1, "Turn"), (7, "Field"), (22, "Change")];
const MORE_WORD: &str = "more";
const EMPTY: &str = "No changes yet.";
const SAVED: &str = "Saved";
const CLEARED: &str = "Log cleared.";
const NOTHING: &str = "Nothing to save.";
const NO_FOLDER: &str = "No folder to save to.";
const NOT_SAVED: &str = "Could not save the log.";
/// The saved file's first line.
const FILE_HEAD: &str = "Turn\tField\tOld\tNew";

/// The most entries the ring holds (7.2.8).
pub const WATCH_CAPACITY: usize = 4096;

/// The groups in the page's order, with their digit keys.
pub const GROUPS: [WatchGroup; 5] = [
    WatchGroup::Party,
    WatchGroup::Stores,
    WatchGroup::Place,
    WatchGroup::Time,
    WatchGroup::Engine,
];

/// A group's label (engine text).
pub const fn group_label(group: WatchGroup) -> &'static str {
    match group {
        WatchGroup::Party => "Party",
        WatchGroup::Stores => "Stores",
        WatchGroup::Place => "Place",
        WatchGroup::Time => "Time",
        WatchGroup::Engine => "Engine",
    }
}

const fn group_index(group: WatchGroup) -> usize {
    match group {
        WatchGroup::Party => 0,
        WatchGroup::Stores => 1,
        WatchGroup::Place => 2,
        WatchGroup::Time => 3,
        WatchGroup::Engine => 4,
    }
}

// ---------------------------------------------------------------------------
// The log.

/// One recorded change.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WatchEntry {
    pub turn: u64,
    /// The host clock, in milliseconds since the app started.
    pub millis: u64,
    pub field: WatchField,
    pub old: WatchValue,
    pub new: WatchValue,
}

/// The watch's ring and its page settings. Exists while `state_watch` is
/// on.
#[derive(Resource, Debug)]
pub struct WatchLog {
    entries: VecDeque<WatchEntry>,
    last: Option<Vec<(WatchField, WatchValue)>>,
    paused: bool,
    shown: [bool; GROUPS.len()],
}

impl Default for WatchLog {
    fn default() -> Self {
        Self {
            entries: VecDeque::with_capacity(WATCH_CAPACITY),
            last: None,
            paused: false,
            // The Engine group is for maintainers and starts unchecked.
            shown: [true, true, true, true, false],
        }
    }
}

impl WatchLog {
    /// Adds one entry; the oldest scrolls off once the ring is full.
    pub fn push(&mut self, entry: WatchEntry) {
        if self.entries.len() == WATCH_CAPACITY {
            self.entries.pop_front();
        }
        self.entries.push_back(entry);
    }

    /// Every entry, oldest first.
    pub fn entries(&self) -> impl DoubleEndedIterator<Item = &WatchEntry> {
        self.entries.iter()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn paused(&self) -> bool {
        self.paused
    }

    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Whether `group` is checked.
    pub fn shows(&self, group: WatchGroup) -> bool {
        self.shown[group_index(group)]
    }

    /// Checks or unchecks the group at `index` of [`GROUPS`].
    pub fn toggle_group(&mut self, index: usize) {
        if let Some(shown) = self.shown.get_mut(index) {
            *shown = !*shown;
        }
    }

    /// Records the changes from the last snapshot to `snapshot`. Nothing is
    /// recorded while paused or for the first snapshot; Engine fields only
    /// while their group is checked.
    pub fn observe(&mut self, turn: u64, millis: u64, snapshot: Vec<(WatchField, WatchValue)>) {
        if let Some(last) = &self.last
            && !self.paused
        {
            let engine = self.shows(WatchGroup::Engine);
            let changes: Vec<_> = watch_diff(last, &snapshot)
                .into_iter()
                .filter(|(field, ..)| engine || field.group() != WatchGroup::Engine)
                .collect();
            for (field, old, new) in changes {
                self.push(WatchEntry {
                    turn,
                    millis,
                    field,
                    old,
                    new,
                });
            }
        }
        self.last = Some(snapshot);
    }

    /// The entries of the checked groups, newest first.
    pub fn visible(&self) -> Vec<&WatchEntry> {
        self.entries
            .iter()
            .rev()
            .filter(|entry| self.shows(entry.field.group()))
            .collect()
    }

    /// The shown log as the saved file holds it: a heading line, then one
    /// tab-separated line per entry, oldest first. Labels and values only.
    pub fn log_text(&self) -> String {
        let mut text = format!("{FILE_HEAD}\n");
        for entry in self.visible().into_iter().rev() {
            text.push_str(&format!(
                "{}\t{}\t{}\t{}\n",
                entry.turn,
                entry.field.label(),
                entry.old.text(),
                entry.new.text()
            ));
        }
        text
    }

    /// Writes [`Self::log_text`] into `folder` as `watch-YYYYMMDD-HHMMSS.txt`
    /// (UTC). Only `folder` is written: it is made if missing, and an
    /// existing file of that name is never replaced.
    pub fn save(&self, folder: &Path, now: SystemTime) -> io::Result<PathBuf> {
        std::fs::create_dir_all(folder)?;
        let stem = watch_file_stem(now);
        let mut path = folder.join(format!("{stem}.txt"));
        let mut n = 1;
        while path.exists() {
            n += 1;
            path = folder.join(format!("{stem}-{n}.txt"));
        }
        std::fs::write(&path, self.log_text())?;
        Ok(path)
    }
}

/// `watch-YYYYMMDD-HHMMSS` for `now`, in UTC.
pub fn watch_file_stem(now: SystemTime) -> String {
    let secs = now
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let (days, rest) = (secs / 86_400, secs % 86_400);
    let (y, m, d) = civil_from_days(days as i64);
    format!(
        "watch-{y:04}{m:02}{d:02}-{:02}{:02}{:02}",
        rest / 3600,
        rest % 3600 / 60,
        rest % 60
    )
}

/// The proleptic Gregorian date of a day count from 1970-01-01.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

/// The folder the host gave for saved logs (`<data_base>/watch/`). Absent:
/// `[S]` says there is nowhere to save.
#[derive(Resource, Clone, Debug, PartialEq, Eq)]
pub struct WatchFolder(pub PathBuf);

/// The watch's recorder (7.3.8): a pure read of the game.
pub(super) fn watch_record(
    visual: Res<VisualState>,
    time: Option<Res<Time<Real>>>,
    mut log: ResMut<WatchLog>,
) {
    let millis = time.map_or(0, |time| time.elapsed().as_millis() as u64);
    log.observe(visual.state.turn, millis, watch_snapshot(&visual.state));
}

// ---------------------------------------------------------------------------
// The Watch face.

/// What a watch key did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WatchOutcome {
    Quiet,
    Say(String),
    Back,
}

/// One key on the Watch face.
pub fn watch_key(
    log: &mut WatchLog,
    key: KeyCode,
    folder: Option<&Path>,
    now: SystemTime,
) -> WatchOutcome {
    match key {
        KeyCode::Escape => WatchOutcome::Back,
        KeyCode::Space => {
            log.toggle_pause();
            WatchOutcome::Quiet
        }
        KeyCode::KeyC => {
            log.clear();
            WatchOutcome::Say(CLEARED.to_string())
        }
        KeyCode::KeyS => WatchOutcome::Say(match folder {
            None => NO_FOLDER.to_string(),
            Some(_) if log.visible().is_empty() => NOTHING.to_string(),
            Some(folder) => match log.save(folder, now) {
                Ok(path) => format!(
                    "{SAVED} {}.",
                    path.file_name().unwrap_or_default().to_string_lossy()
                ),
                Err(_) => NOT_SAVED.to_string(),
            },
        }),
        KeyCode::Digit1 | KeyCode::Numpad1 => toggled(log, 0),
        KeyCode::Digit2 | KeyCode::Numpad2 => toggled(log, 1),
        KeyCode::Digit3 | KeyCode::Numpad3 => toggled(log, 2),
        KeyCode::Digit4 | KeyCode::Numpad4 => toggled(log, 3),
        KeyCode::Digit5 | KeyCode::Numpad5 => toggled(log, 4),
        _ => WatchOutcome::Quiet,
    }
}

fn toggled(log: &mut WatchLog, index: usize) -> WatchOutcome {
    log.toggle_group(index);
    WatchOutcome::Quiet
}

/// The face's rows, relative to the panel's top: the help above the first
/// divider, the entries, then the groups and the result line.
pub const WATCH_DIVIDERS: [u32; 2] = [2, 18];
const HELP_ROW: u32 = 1;
const HEAD_ROW: u32 = 3;
const FIRST_ROW: u32 = 4;
/// The box's last entry row; the row under it stays empty above the rule.
const LAST_ROW: u32 = 16;
/// The groups' first row and their two columns, as the main page's locks.
const GROUP_ROW: u32 = 19;
const GROUP_COLS: [u32; 2] = [1, 20];
pub const WATCH_RESULT_ROW: u32 = 22;
/// The change column, and its width before an entry takes a second row.
const CHANGE_COL: u32 = 22;
const CHANGE_WIDTH: usize = 15;
const CONTINUED_WIDTH: usize = 30;

/// The two values as the page shows them: a shared date before the time
/// is dropped, so a clock change reads `09:14 > 09:15`.
fn change_text(old: &WatchValue, new: &WatchValue) -> String {
    let (old, new) = (old.text(), new.text());
    if let (Some((old_head, old_tail)), Some((new_head, new_tail))) =
        (old.rsplit_once(' '), new.rsplit_once(' '))
        && old_head == new_head
        && !old_head.contains(' ')
    {
        return format!("{old_tail} > {new_tail}");
    }
    format!("{old} > {new}")
}

fn clipped(text: &str, width: usize) -> String {
    text.chars().take(width).collect()
}

/// Every run of the Watch face except the result line. Column 0 is the
/// box's first cell, as on the edit face.
pub fn watch_runs(log: &WatchLog) -> Vec<EditRun> {
    let mut runs = Vec::new();
    let mut put = |col: u32, row: u32, text: String, ink: Ink| {
        runs.push(EditRun {
            col,
            row,
            text,
            ink,
        });
    };
    put(1, HELP_ROW, WATCH_HELP.to_string(), Ink::Help);
    for (col, head) in WATCH_HEADS {
        put(col, HEAD_ROW, head.to_string(), Ink::Help);
    }
    let visible = log.visible();
    if visible.is_empty() {
        put(1, FIRST_ROW, EMPTY.to_string(), Ink::Help);
    }
    let mut row = FIRST_ROW;
    let mut drawn = 0;
    for entry in &visible {
        let change = change_text(&entry.old, &entry.new);
        let rows = if change.len() > CHANGE_WIDTH { 2 } else { 1 };
        // The last row is kept for the "more" count when entries remain.
        let room = if drawn + 1 < visible.len() {
            LAST_ROW
        } else {
            LAST_ROW + 1
        };
        if row + rows > room {
            break;
        }
        put(1, row, clipped(&entry.turn.to_string(), 5), Ink::Text);
        put(7, row, clipped(&entry.field.label(), 14), Ink::Text);
        if rows == 1 {
            put(CHANGE_COL, row, change, Ink::Text);
        } else {
            put(7, row + 1, clipped(&change, CONTINUED_WIDTH), Ink::Text);
        }
        row += rows;
        drawn += 1;
    }
    if drawn < visible.len() {
        put(
            CHANGE_COL,
            LAST_ROW,
            format!("({} {MORE_WORD})", visible.len() - drawn),
            Ink::Help,
        );
    }
    for (i, group) in GROUPS.iter().enumerate() {
        let (row, col) = (GROUP_ROW + (i / 2) as u32, GROUP_COLS[i % 2]);
        let mark = if log.shows(*group) { 'x' } else { ' ' };
        put(
            col,
            row,
            format!("[{mark}] {} {}", i + 1, group_label(*group)),
            Ink::Text,
        );
    }
    runs
}

/// The face's title tab.
pub fn watch_title(log: Option<&WatchLog>) -> &'static str {
    if log.is_some_and(WatchLog::paused) {
        WATCH_TITLE_PAUSED
    } else {
        WATCH_TITLE
    }
}

#[cfg(test)]
#[path = "watch_tests.rs"]
mod tests;

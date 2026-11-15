//! The change watch's log, face and saved file (design 7.2.8, 7.3.8;
//! build plan M8-05).

use std::time::{Duration, SystemTime};

use u5_runtime::cheats::{WatchField, WatchValue};
use u5_runtime::test_fixtures::unique_scratch_dir;

use super::*;

fn num(field: WatchField, value: i64) -> (WatchField, WatchValue) {
    (field, WatchValue::Num(value))
}

fn text(field: WatchField, value: &str) -> (WatchField, WatchValue) {
    (field, WatchValue::Text(value.to_string()))
}

fn entry(turn: u64, field: WatchField, old: i64, new: i64) -> WatchEntry {
    WatchEntry {
        turn,
        millis: 0,
        field,
        old: WatchValue::Num(old),
        new: WatchValue::Num(new),
    }
}

#[test]
fn observe_records_the_diff_of_synthetic_snapshots() {
    let mut log = WatchLog::default();
    let first = vec![
        num(WatchField::Food, 211),
        num(WatchField::MemberHp(0), 187),
        text(WatchField::MemberStatus(0), "G"),
        text(WatchField::Wind, "north"),
    ];
    log.observe(1040, 0, first.clone());
    assert!(log.is_empty(), "the first snapshot is the baseline");
    log.observe(1040, 5, first.clone());
    assert!(log.is_empty(), "an unchanged frame records nothing");
    let mut second = first;
    second[0] = num(WatchField::Food, 210);
    second[1] = num(WatchField::MemberHp(0), 163);
    second[2] = text(WatchField::MemberStatus(0), "P");
    second.push(num(WatchField::Hull, 50));
    log.observe(1042, 9, second);
    let got: Vec<_> = log
        .entries()
        .map(|e| (e.turn, e.field, e.old.text(), e.new.text()))
        .collect();
    assert_eq!(
        got,
        [
            (1042, WatchField::Hull, "-".into(), "50".into()),
            (1042, WatchField::Food, "211".into(), "210".into()),
            (1042, WatchField::MemberStatus(0), "G".into(), "P".into()),
            (1042, WatchField::MemberHp(0), "187".into(), "163".into()),
        ]
    );
    // Newest first on the face.
    assert_eq!(log.visible()[0].field, WatchField::MemberHp(0));
}

#[test]
fn the_engine_group_starts_unchecked_and_is_only_recorded_while_checked() {
    let mut log = WatchLog::default();
    assert!(!log.shows(WatchGroup::Engine));
    for group in [
        WatchGroup::Party,
        WatchGroup::Stores,
        WatchGroup::Place,
        WatchGroup::Time,
    ] {
        assert!(log.shows(group));
    }
    log.observe(
        1,
        0,
        vec![num(WatchField::Prng, 1), num(WatchField::Turn, 1)],
    );
    log.observe(
        2,
        0,
        vec![num(WatchField::Prng, 2), num(WatchField::Turn, 2)],
    );
    assert!(log.is_empty());
    log.toggle_group(4);
    log.observe(
        3,
        0,
        vec![num(WatchField::Prng, 3), num(WatchField::Turn, 3)],
    );
    assert_eq!(log.len(), 2);
    // An unchecked group is hidden, not dropped.
    log.observe(
        3,
        0,
        vec![num(WatchField::Prng, 3), num(WatchField::Gold, 1)],
    );
    log.observe(
        3,
        0,
        vec![num(WatchField::Prng, 3), num(WatchField::Gold, 2)],
    );
    log.toggle_group(1);
    assert!(log.visible().iter().all(|e| e.field != WatchField::Gold));
    log.toggle_group(1);
    assert_eq!(log.visible()[0].field, WatchField::Gold);
}

#[test]
fn pause_stops_recording_and_clear_empties_the_log() {
    let mut log = WatchLog::default();
    log.observe(1, 0, vec![num(WatchField::Gold, 1)]);
    log.toggle_pause();
    log.observe(2, 0, vec![num(WatchField::Gold, 2)]);
    assert!(log.is_empty());
    log.toggle_pause();
    log.observe(3, 0, vec![num(WatchField::Gold, 3)]);
    assert_eq!(log.len(), 1, "resumed from the latest frame");
    assert_eq!(log.entries().next().unwrap().old, WatchValue::Num(2));
    log.clear();
    assert!(log.is_empty());
}

#[test]
fn the_ring_wraps_at_4096() {
    let mut log = WatchLog::default();
    for turn in 0..(WATCH_CAPACITY as u64 + 10) {
        log.push(entry(turn, WatchField::Gold, 0, 1));
    }
    assert_eq!(log.len(), WATCH_CAPACITY);
    assert_eq!(
        log.entries().next().unwrap().turn,
        10,
        "the oldest scrolled off"
    );
    assert_eq!(
        log.entries().last().unwrap().turn,
        WATCH_CAPACITY as u64 + 9
    );
}

#[test]
fn the_file_name_is_the_utc_stamp() {
    let at = SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    assert_eq!(watch_file_stem(at), "watch-20231114-221320");
    let leap = SystemTime::UNIX_EPOCH + Duration::from_secs(951_782_400);
    assert_eq!(watch_file_stem(leap), "watch-20000229-000000");
}

#[test]
fn a_saved_log_holds_labels_and_values_only_and_only_in_the_given_folder() {
    let root = unique_scratch_dir("watch-save");
    let folder = root.join("watch");
    let mut log = WatchLog::default();
    log.push(entry(7, WatchField::MemberHp(0), 30, 12));
    log.push(WatchEntry {
        turn: 8,
        millis: 99,
        field: WatchField::Clock,
        old: WatchValue::Text("139-4-2 09:14".into()),
        new: WatchValue::Text("139-4-2 09:15".into()),
    });
    let at = SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let path = log.save(&folder, at).unwrap();
    assert_eq!(path, folder.join("watch-20231114-221320.txt"));
    let again = log.save(&folder, at).unwrap();
    assert_eq!(
        again,
        folder.join("watch-20231114-221320-2.txt"),
        "never replaced"
    );
    // Only the folder was written.
    let top: Vec<_> = std::fs::read_dir(&root)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(top, ["watch"]);
    let text = std::fs::read_to_string(&path).unwrap();
    assert_eq!(
        text,
        "Turn\tField\tOld\tNew\n7\tParty 1 HP\t30\t12\n8\tClock\t139-4-2 09:14\t139-4-2 09:15\n"
    );
    for line in text.lines() {
        assert_eq!(line.split('\t').count(), 4, "{line}");
    }
    // The key saves the shown log; with no folder it says so.
    let keyed = root.join("keyed");
    assert!(matches!(
        watch_key(&mut log, KeyCode::KeyS, Some(&keyed), at),
        WatchOutcome::Say(line) if line.starts_with("Saved watch-")
    ));
    assert_eq!(
        watch_key(&mut log, KeyCode::KeyS, None, at),
        WatchOutcome::Say(NO_FOLDER.to_string())
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn the_face_shows_newest_first_with_a_more_count() {
    let mut log = WatchLog::default();
    for turn in 0..40 {
        log.push(entry(turn, WatchField::Gold, turn as i64, turn as i64 + 1));
    }
    let runs = watch_runs(&log);
    let at = |row: u32, col: u32| {
        runs.iter()
            .find(|r| r.row == row && r.col == col)
            .map(|r| r.text.clone())
    };
    assert_eq!(at(4, 1).as_deref(), Some("39"));
    assert_eq!(at(4, 22).as_deref(), Some("39 > 40"));
    assert_eq!(at(15, 1).as_deref(), Some("28"));
    assert_eq!(at(16, 22).as_deref(), Some("(28 more)"));
    assert_eq!(at(19, 20).as_deref(), Some("[x] 2 Stores"));
    assert_eq!(at(21, 1).as_deref(), Some("[ ] 5 Engine"));
    // A shared date is dropped from a clock change; a long change takes a
    // second row under the field.
    let mut log = WatchLog::default();
    log.push(WatchEntry {
        turn: 1,
        millis: 0,
        field: WatchField::Area,
        old: WatchValue::Text("Location 2 floor 0".into()),
        new: WatchValue::Text("Britannia".into()),
    });
    log.push(WatchEntry {
        turn: 2,
        millis: 0,
        field: WatchField::Clock,
        old: WatchValue::Text("139-4-2 09:14".into()),
        new: WatchValue::Text("139-4-2 09:15".into()),
    });
    let runs = watch_runs(&log);
    let texts: Vec<_> = runs
        .iter()
        .map(|r| (r.row, r.col, r.text.as_str()))
        .collect();
    assert!(texts.contains(&(4, 22, "09:14 > 09:15")));
    assert!(texts.contains(&(6, 7, "Location 2 floor 0 > Britannia")));
    // Every run stays inside the box.
    for run in &runs {
        assert!(run.col + run.text.len() as u32 <= 37, "{run:?}");
    }
}

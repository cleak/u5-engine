//! Test-only recorder for the grow-left read-only accessors (Phase B
//! M1-08, design 2.3.1): `stats_panel_counter_text` and
//! `stats_panel_date_text` must return exactly what the faithful panel
//! paints. The headless status-surface render compares the two on every
//! frame that paints the ordinary panel rows while recording is on, and a
//! test runs the frame and route suites over it. Nothing here is compiled
//! outside tests.

use std::cell::RefCell;

#[derive(Default)]
struct Recorder {
    checked: usize,
    mismatches: Vec<String>,
}

thread_local! {
    static RECORDER: RefCell<Option<Recorder>> = const { RefCell::new(None) };
}

/// Start recording on this thread, dropping anything recorded before.
pub(crate) fn start() {
    RECORDER.with(|cell| *cell.borrow_mut() = Some(Recorder::default()));
}

/// Stop recording; returns how many frames were compared and every
/// mismatch found.
pub(crate) fn finish() -> (usize, Vec<String>) {
    RECORDER.with(|cell| {
        cell.borrow_mut()
            .take()
            .map(|r| (r.checked, r.mismatches))
            .unwrap_or_default()
    })
}

pub(crate) fn recording() -> bool {
    RECORDER.with(|cell| cell.borrow().is_some())
}

/// Compare one painted counter/date pair (screen rows 8 and 9, columns
/// 24..=39) with the accessors' text.
pub(crate) fn note(painted: [&str; 2], accessor: [&str; 2]) {
    RECORDER.with(|cell| {
        if let Some(recorder) = cell.borrow_mut().as_mut() {
            recorder.checked += 1;
            for (row, (painted, accessor)) in ["counter", "date"]
                .into_iter()
                .zip(painted.into_iter().zip(accessor))
            {
                if painted.trim_end() != accessor.trim_end() {
                    recorder
                        .mismatches
                        .push(format!("{row}: painted {painted:?}, accessor {accessor:?}"));
                }
            }
        }
    });
}

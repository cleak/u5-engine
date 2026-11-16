//! Clue highlighting in the Bevy shell: the `[highlight]` step of the
//! presentation pipeline (Phase B design 8.3.10; build plan M6-05).
//!
//! ```text
//! EnhSet::Layout   highlight_plan: the window rows recomputed from
//!                  &PlayState, the session's rows found, the spans of
//!                  u5_runtime::enhancements::clues::highlight_spans and
//!                  the glyph each marked cell should show -> HighlightPlan
//! EnhSet::Present  [highlight] (StepOrder::HIGHLIGHT, before grow-left and
//!                  prescale): the cell check against the faithful image,
//!                  then the marks on the presented copy
//! ```
//!
//! - **Rows.** The faithful renderer keeps its rows in a local, so the plan
//!   recomputes them with `message_log_from_entries` and
//!   `layout_message_window_with_continuation`. They can differ from the
//!   painted ones (a shop screen, a live input row), so before marking the
//!   step compares every marked cell of the faithful image with the glyph
//!   `paint_fixed_cell_glyph` would draw there; on any mismatch it marks
//!   nothing in that frame.
//! - **Session rows.** Only rows the current conversation printed, and not
//!   the player's own echo rows, are eligible: the plan follows the
//!   transcript with a [`TranscriptCursor`] from the frame the session
//!   opened and matches the session's own rows from the bottom of the
//!   window up.
//! - **Marks** lie on the bottom pixel row of each cell in pens of the
//!   game's EGA palette, skipping the glyph's own pixels there, so glyphs,
//!   wrapping and rows never move: dotted (every other pixel, pen 15) for
//!   asked and the same, dashed (three of four, pen 6) for asked and may
//!   differ, sparse (one of four, pen 8) for heard elsewhere and dotted in
//!   pen 10, offset by one, for `new`. Style `tint` repaints the cell's
//!   background in one pen per state (1, 6, 8, 2) and the glyph over it in
//!   pen 15.
//! - **Revision.** The step's revision is the plan's, which changes only
//!   when the marked cells do: a new asked set, or a test input crossing a
//!   threshold (8.3.13), repaints with the faithful image unchanged.
//! - **Asked set.** The clue log's entries for this dialog; with `clues`
//!   off, the observer's session-only set.
//!
//! **Zero footprint.** The step, the plan and its system exist only while
//! `clue_highlight` is not `off`; without them no `record_dependence` scan
//! runs. The step writes only the presented copy: the faithful image, its
//! upload sites and its hash are untouched.

use std::sync::{Arc, Mutex};

use bevy::prelude::*;
use u5_runtime::enhancements::clues::{
    Asked, ClueEntry, ConversationSnapshot, DialogKey, HighlightRow, HighlightSession, Mark, Span,
    highlight_spans, is_prompt_or_echo_row, topic_keyword,
};
use u5_runtime::enhancements::{
    ClueHighlight, FeatureToggles, HighlightStyle, TranscriptCursor, TranscriptDiff, Unanchored,
};
use u5_runtime::{
    CHROME_RULE_INDEX, EGA_PALETTE_RGB, FixedCellFont, LiveRowKind, MessageEntry, MessageWindowRow,
    PlayState, TlkGlyphFont, TlkRenderedGlyph, combat_prompt_row_follows_history,
    layout_message_window_with_continuation, message_log_from_entries, paint_fixed_cell_glyph,
};

use super::clues::{ClueLogResource, ClueObserver, HIGHLIGHT, TlkLiveCache};
use crate::VisualState;
use crate::enh::present::{
    PresentInput, PresentStep, PresentSteps, PresentedFrame, StepOrder, install_present_pipeline,
    register_present_step,
};
use crate::enh::{EnhLaunch, EnhSet, Toggles, VisualLaunchConfig};

/// Cell side in pixels (`systems/text-output.md §10.1`).
const CELL: usize = 8;
/// The faithful frame's size.
const FRAME: (usize, usize) = (320, 200);

/// What the step draws: the marked cells and the glyph each should show.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HighlightPlan {
    /// Changes whenever `cells` or `style` do.
    pub revision: u64,
    pub style: HighlightStyle,
    pub cells: Vec<PlanCell>,
}

/// One marked cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlanCell {
    pub column: u8,
    pub row: u8,
    pub mark: Mark,
    /// The glyph's pixels, one byte per pixel row, bit 7 the leftmost.
    pub glyph: [u8; CELL],
}

/// The plan shared with the step, and what the plan system keeps between
/// frames. Exists only while `clue_highlight` is not `off`.
#[derive(Resource)]
pub struct HighlightFeed {
    plan: Arc<Mutex<HighlightPlan>>,
    mode: ClueHighlight,
    style: HighlightStyle,
    /// The transcript as the last frame left it.
    cursor: Option<TranscriptCursor>,
    /// The conversation followed and the index of its first entry.
    session: Option<(DialogKey, usize)>,
    /// The keywords asked of other people, for the dialog they were
    /// gathered for.
    elsewhere: Option<(DialogKey, Vec<Vec<u8>>)>,
}

impl HighlightFeed {
    fn new(mode: ClueHighlight, style: HighlightStyle) -> Self {
        Self {
            plan: Arc::new(Mutex::new(HighlightPlan {
                style,
                ..HighlightPlan::default()
            })),
            mode,
            style,
            cursor: None,
            session: None,
            elsewhere: None,
        }
    }

    /// The plan as the step will draw it.
    pub fn plan(&self) -> HighlightPlan {
        self.plan
            .lock()
            .map(|plan| plan.clone())
            .unwrap_or_default()
    }

    fn step(&self) -> HighlightStep {
        HighlightStep {
            plan: self.plan.clone(),
        }
    }

    fn set_cells(&self, cells: Vec<PlanCell>) {
        let Ok(mut plan) = self.plan.lock() else {
            return;
        };
        if plan.cells != cells || plan.style != self.style {
            plan.cells = cells;
            plan.style = self.style;
            plan.revision = plan.revision.wrapping_add(1);
        }
    }
}

/// The `[highlight]` step.
pub struct HighlightStep {
    plan: Arc<Mutex<HighlightPlan>>,
}

impl PresentStep for HighlightStep {
    fn order(&self) -> StepOrder {
        StepOrder::HIGHLIGHT
    }

    fn revision(&mut self, _input: &PresentInput) -> u64 {
        self.plan.lock().map_or(0, |plan| plan.revision)
    }

    fn apply(&mut self, input: &PresentInput, frame: &mut PresentedFrame) {
        let Ok(plan) = self.plan.lock() else {
            return;
        };
        paint_plan(&plan, input.faithful, input.size, frame);
    }
}

/// The mode and style at launch. A key switched on in play without a mode
/// of its own takes `asked`.
fn launch_mode(launch: Option<&VisualLaunchConfig>) -> (ClueHighlight, HighlightStyle) {
    let clues = launch.map(|launch| launch.enhancements.journal().clues);
    let mode = clues
        .map(|clues| clues.highlight)
        .filter(|mode| *mode != ClueHighlight::Off)
        .unwrap_or(ClueHighlight::Asked);
    let style = clues.map(|clues| clues.highlight_style).unwrap_or_default();
    (mode, style)
}

/// Whether the step is wanted under `toggles`.
pub fn highlight_wanted(toggles: FeatureToggles) -> bool {
    toggles.on(HIGHLIGHT)
}

/// Adds the step at launch while `clue_highlight` is on, and the system
/// that follows the key in play while the clue plugin is in the app (it is
/// added only when `clues` or `clue_highlight` is on at launch).
pub(crate) fn add_highlight(app: &mut App, launch: &VisualLaunchConfig) {
    let toggles = launch.enhancements.toggles;
    if !super::clues::observer_wanted(toggles) {
        return;
    }
    if highlight_wanted(toggles) {
        let (mode, style) = launch_mode(Some(launch));
        let feed = HighlightFeed::new(mode, style);
        let step = feed.step();
        app.insert_resource(feed);
        register_present_step(app, step);
    } else {
        // The key may come on in play: the step list has to be there. With
        // no step the host stays inactive.
        install_present_pipeline(app);
    }
    app.add_systems(
        Update,
        follow_highlight
            .in_set(EnhSet::Apply)
            .after(crate::enh::apply_pending_changes)
            .run_if(resource_changed::<Toggles>),
    )
    .add_systems(
        Update,
        highlight_plan
            .in_set(EnhSet::Layout)
            .run_if(resource_exists::<HighlightFeed>)
            .run_if(resource_exists::<VisualState>),
    );
}

/// Registers or removes the step as `clue_highlight` turns on and off.
fn follow_highlight(
    mut commands: Commands,
    toggles: Res<Toggles>,
    launch: Option<Res<EnhLaunch>>,
    feed: Option<Res<HighlightFeed>>,
    steps: Option<ResMut<PresentSteps>>,
) {
    match (highlight_wanted(toggles.0), feed.is_some()) {
        (true, false) => {
            let (mode, style) = launch_mode(launch.as_deref().map(|launch| &launch.0));
            let feed = HighlightFeed::new(mode, style);
            if let Some(mut steps) = steps {
                steps.register(feed.step());
            }
            commands.insert_resource(feed);
        }
        (false, true) => {
            commands.remove_resource::<HighlightFeed>();
            if let Some(mut steps) = steps {
                steps.remove(StepOrder::HIGHLIGHT);
            }
        }
        _ => {}
    }
}

/// The plan for this frame.
fn highlight_plan(
    mut feed: ResMut<HighlightFeed>,
    visual: Res<VisualState>,
    log: Option<Res<ClueLogResource>>,
    observer: Option<Res<ClueObserver>>,
    mut cache: Option<ResMut<TlkLiveCache>>,
) {
    let feed = &mut *feed;
    let state = &visual.state;
    let entries = state.message_entries();
    let snapshot = ConversationSnapshot::of(state);

    // Follow the transcript: where this conversation's entries start.
    let diff = feed
        .cursor
        .map(|cursor| (cursor.diff_state(state), cursor.len()));
    feed.cursor = Some(TranscriptCursor::of(state));
    let retained_start = |start: usize| match diff {
        Some((TranscriptDiff::Unchanged, _)) => Some(start),
        Some((
            TranscriptDiff::Anchored {
                first_new,
                replaced_last,
            },
            old_len,
        )) => {
            let kept = first_new + usize::from(replaced_last);
            Some(start.saturating_sub(old_len.saturating_sub(kept)))
        }
        // Every entry is new since the last frame.
        Some((TranscriptDiff::Unanchored(Unanchored::Cleared | Unanchored::Capped), _)) => Some(0),
        Some((TranscriptDiff::Unanchored(_), _)) | None => None,
    };
    feed.session = match (snapshot, feed.session) {
        (None, _) => None,
        (Some(now), Some((dialog, start))) if dialog == now.dialog => {
            Some((dialog, retained_start(start).unwrap_or(entries.len())))
        }
        // A conversation opened since the last frame: what this frame added
        // is its opening.
        (Some(now), _) => {
            let start = match diff {
                Some((TranscriptDiff::Anchored { first_new, .. }, _)) => first_new,
                Some((TranscriptDiff::Unanchored(Unanchored::Cleared), _)) => 0,
                _ => entries.len(),
            };
            Some((now.dialog, start.min(entries.len())))
        }
    };

    // A spell's flash holds the window on older rows: nothing to mark.
    let held = state.displayed_message_entries().len() != entries.len();
    let (Some(now), Some((dialog, start)), Some(session), false) = (
        snapshot,
        feed.session,
        state.active_conversation.as_ref(),
        held,
    ) else {
        feed.set_cells(Vec::new());
        return;
    };

    // Who was asked what.
    let here: Vec<ClueEntry> = match log.as_deref() {
        Some(log) => log.log().entries_for(dialog).copied().collect(),
        None => observer
            .as_deref()
            .map(|observer| {
                observer
                    .asked_entries()
                    .iter()
                    .filter(|entry| entry.dialog == dialog)
                    .copied()
                    .collect()
            })
            .unwrap_or_default(),
    };
    let wants_elsewhere = matches!(
        feed.mode,
        ClueHighlight::AskedElsewhere | ClueHighlight::New
    );
    if !wants_elsewhere {
        feed.elsewhere = None;
    } else if let (Some(log), Some(cache)) = (log.as_ref(), cache.as_deref_mut()) {
        let stale = feed
            .elsewhere
            .as_ref()
            .is_none_or(|(for_dialog, _)| *for_dialog != dialog)
            || log.is_changed();
        if stale {
            feed.elsewhere = Some((
                dialog,
                elsewhere_keywords(log.log().entries(), dialog, |d| {
                    cache.blob(&visual.game_dir, d).map(<[Vec<u8>]>::to_vec)
                }),
            ));
        }
    }
    let elsewhere = feed
        .elsewhere
        .as_ref()
        .map_or(&[][..], |(_, words)| words.as_slice());

    let layout_rows = window_rows(state, entries);
    let session_rows = window_rows_of_session(state, &entries[start.min(entries.len())..]);
    let rows = eligible_rows(&layout_rows, &session_rows);
    let highlight = HighlightSession {
        fields: &session.fields,
        decoded: &session.decoded_fields,
        phase: session.phase,
        now: now.inputs,
    };
    let spans = highlight_spans(
        &rows,
        Some(&highlight),
        &Asked {
            here: &here,
            elsewhere,
        },
        feed.mode,
    );
    let cells = plan_cells(&spans, &layout_rows, &visual.text_font, &visual.rune_font);
    feed.set_cells(cells);
}

/// The keywords of the pairs asked of people other than `dialog`, once
/// each, read from their blobs.
fn elsewhere_keywords(
    entries: &[ClueEntry],
    dialog: DialogKey,
    mut blob: impl FnMut(DialogKey) -> Option<Vec<Vec<u8>>>,
) -> Vec<Vec<u8>> {
    let mut words: Vec<Vec<u8>> = Vec::new();
    let mut others: Vec<(DialogKey, _)> = entries
        .iter()
        .filter(|entry| entry.dialog != dialog)
        .map(|entry| (entry.dialog, entry.record))
        .collect();
    others.sort_by_key(|(d, _)| *d);
    let mut current: Option<(DialogKey, Option<Vec<Vec<u8>>>)> = None;
    for (d, record) in others {
        if current.as_ref().is_none_or(|(c, _)| *c != d) {
            current = Some((d, blob(d)));
        }
        let Some((_, Some(fields))) = current.as_ref() else {
            continue;
        };
        if let Some(word) = topic_keyword(fields, record)
            && !words.contains(&word)
        {
            words.push(word);
        }
    }
    words
}

/// The rows the window shows for `entries`, composed as the faithful
/// renderer composes them in a conversation: the transcript, the unflushed
/// slot, an open prompt and the live row.
fn window_rows(state: &PlayState, entries: &[MessageEntry]) -> Vec<MessageWindowRow> {
    let keep = |text: &str| (!text.trim().is_empty()).then(|| text.to_string());
    let mut log = message_log_from_entries(entries, keep);
    log.set_top_offset(usize::from(state.message_window_top_offset()));
    if let Some(text) = state.displayed_unflushed_message().and_then(keep) {
        log.push_output(&text);
    }
    let open_prompt = state.open_prompt_line();
    let echo = state.typed_prompt_echo();
    let live_row =
        (!state.message_window_live_row_suppressed()).then(|| echo.as_deref().unwrap_or(""));
    let kind = if echo.is_some() {
        LiveRowKind::Continuation
    } else {
        LiveRowKind::CommandRow
    };
    layout_message_window_with_continuation(
        &log,
        live_row,
        open_prompt.as_deref(),
        combat_prompt_row_follows_history(state),
        kind,
    )
    .rows
}

/// The session's own rows: its entries and the unflushed slot, wrapped as
/// the window wraps them, without a live row.
fn window_rows_of_session(
    state: &PlayState,
    entries: &[MessageEntry],
) -> Vec<Vec<TlkRenderedGlyph>> {
    let keep = |text: &str| (!text.trim().is_empty()).then(|| text.to_string());
    let mut log = message_log_from_entries(entries, keep);
    if let Some(text) = state.displayed_unflushed_message().and_then(keep) {
        log.push_output(&text);
    }
    log.lines()
        .iter()
        .map(|line| line.glyphs.clone())
        .filter(|glyphs| !glyphs.is_empty())
        .collect()
}

/// The window's rows with the session's marked eligible: from the bottom
/// up, each non-empty row that equals the session's next row up. Rows
/// under the first match (the live row) are passed over; the first row
/// that differs after a match ends the session's part. Echo and prompt
/// rows match but are never eligible.
fn eligible_rows<'a>(
    layout: &'a [MessageWindowRow],
    session: &[Vec<TlkRenderedGlyph>],
) -> Vec<HighlightRow<'a>> {
    let mut eligible = vec![false; layout.len()];
    let mut next = session.len();
    let mut matched = false;
    for (index, row) in layout.iter().enumerate().rev() {
        if row.glyphs.is_empty() {
            continue;
        }
        if next > 0 && session[next - 1] == row.glyphs {
            next -= 1;
            matched = true;
            eligible[index] = !row.prefixed && !is_prompt_or_echo_row(&row.text);
        } else if matched {
            break;
        }
    }
    layout
        .iter()
        .zip(eligible)
        .map(|(row, eligible)| HighlightRow {
            row: row.row,
            column: row.column,
            glyphs: &row.glyphs,
            eligible,
        })
        .collect()
}

/// Each span's cells with the glyph the faithful renderer paints there.
fn plan_cells(
    spans: &[Span],
    rows: &[MessageWindowRow],
    ibm: &FixedCellFont,
    runes: &FixedCellFont,
) -> Vec<PlanCell> {
    let mut cells = Vec::new();
    for span in spans {
        let Some(row) = rows.iter().find(|row| row.row == span.row) else {
            continue;
        };
        for column in span.column..span.column.saturating_add(span.len) {
            let Some(glyph) = row.glyphs.get(usize::from(column - row.column)) else {
                continue;
            };
            let font = match glyph.font {
                TlkGlyphFont::Ordinary => ibm,
                TlkGlyphFont::Runic => runes,
            };
            cells.push(PlanCell {
                column,
                row: span.row,
                mark: span.mark,
                glyph: glyph_mask(font, glyph.byte),
            });
        }
    }
    cells
}

/// The pixels `paint_fixed_cell_glyph` lights for `code`.
fn glyph_mask(font: &FixedCellFont, code: u8) -> [u8; CELL] {
    let mut scratch = [0u8; CELL * CELL * 4];
    paint_fixed_cell_glyph(
        &mut scratch,
        CELL,
        CELL,
        font,
        code,
        0,
        0,
        CHROME_RULE_INDEX,
    );
    let mut mask = [0u8; CELL];
    for (y, bits) in mask.iter_mut().enumerate() {
        for x in 0..CELL {
            let at = (y * CELL + x) * 4;
            if scratch[at..at + 4] != [0, 0, 0, 0] {
                *bits |= 0x80 >> x;
            }
        }
    }
    mask
}

fn pen(index: u8) -> [u8; 4] {
    let [r, g, b] = EGA_PALETTE_RGB[usize::from(index)];
    [r, g, b, 0xFF]
}

/// The pens of the marks and of the tint backgrounds.
fn mark_pen(mark: Mark) -> u8 {
    match mark {
        Mark::Same => CHROME_RULE_INDEX,
        Mark::MayDiffer => 6,
        Mark::Elsewhere => 8,
        Mark::New => 10,
    }
}

fn tint_pen(mark: Mark) -> u8 {
    match mark {
        Mark::Same => 1,
        Mark::MayDiffer => 6,
        Mark::Elsewhere => 8,
        Mark::New => 2,
    }
}

/// Whether the mark lights absolute pixel column `x` of a bottom row.
fn lights(mark: Mark, x: usize) -> bool {
    match mark {
        Mark::Same => x.is_multiple_of(2),
        Mark::MayDiffer => x % 4 != 3,
        Mark::Elsewhere => x.is_multiple_of(4),
        Mark::New => x % 2 == 1,
    }
}

/// The cell check, then the marks. Returns whether anything was drawn.
/// The plan is checked against `faithful` and drawn on `frame`, which must
/// still be the faithful size.
pub fn paint_plan(
    plan: &HighlightPlan,
    faithful: &[u8],
    size: UVec2,
    frame: &mut PresentedFrame,
) -> bool {
    let (w, h) = FRAME;
    if plan.cells.is_empty()
        || size != UVec2::new(w as u32, h as u32)
        || frame.size != size
        || faithful.len() != w * h * 4
    {
        return false;
    }
    let ink = pen(CHROME_RULE_INDEX);
    let paper = pen(0);
    let at = |cell: &PlanCell, x: usize, y: usize| {
        ((usize::from(cell.row) * CELL + y) * w + usize::from(cell.column) * CELL + x) * 4
    };
    let in_frame =
        |cell: &PlanCell| usize::from(cell.column) < w / CELL && usize::from(cell.row) < h / CELL;
    let matches = plan.cells.iter().all(|cell| {
        in_frame(cell)
            && (0..CELL).all(|y| {
                (0..CELL).all(|x| {
                    let lit = cell.glyph[y] & (0x80 >> x) != 0;
                    let i = at(cell, x, y);
                    faithful[i..i + 4] == if lit { ink } else { paper }
                })
            })
    });
    if !matches {
        return false;
    }
    for cell in &plan.cells {
        match plan.style {
            HighlightStyle::Dotted => {
                let y = CELL - 1;
                let colour = pen(mark_pen(cell.mark));
                for x in 0..CELL {
                    let lit = cell.glyph[y] & (0x80 >> x) != 0;
                    if !lit && lights(cell.mark, usize::from(cell.column) * CELL + x) {
                        let i = at(cell, x, y);
                        frame.rgba[i..i + 4].copy_from_slice(&colour);
                    }
                }
            }
            HighlightStyle::Tint => {
                let back = pen(tint_pen(cell.mark));
                for y in 0..CELL {
                    for x in 0..CELL {
                        let lit = cell.glyph[y] & (0x80 >> x) != 0;
                        let i = at(cell, x, y);
                        frame.rgba[i..i + 4].copy_from_slice(if lit { &ink } else { &back });
                    }
                }
            }
        }
    }
    true
}

#[cfg(test)]
#[path = "highlight/tests.rs"]
mod tests;

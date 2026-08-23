//! `systems/intro.md §11` acknowledgements (`A`) presentation geometry
//! and step sequencing.
//!
//! Resolved by `cleak/u5-spec#72` (closed; answered against spec heads
//! `8192d67` and `3bbcd5e`). The acknowledgement screen is **artwork,
//! not text**: the `STARTSC` archive holds three records whose credit
//! lines are drawn into the bitmap, so nothing here typesets a credit
//! string and nothing may be authored in the engine.
//!
//! `#72` also **withdrew in full** the earlier "bottom-up entry wipe /
//! top-down exit wipe with horizontal slabs" model that this engine
//! previously carried in `require_acknowledgements_contract`'s doc
//! comment. There are no horizontal slabs in either direction. The
//! real sequence is the five moving phases of `§11.2`, reproduced here
//! as pure geometry:
//!
//! 1. **Compose** — record `1` onto the *hidden* surface at `(16, 63)`.
//! 2. **Rise** — 137 unpaced steps, both pillars climbing the centre.
//! 3. **Part** — 18 steps at an 8-pixel stride, one BIOS tick each.
//! 4. **Close** — 18 mirrored steps, one BIOS tick each.
//! 5. **Sink** — 137 unpaced steps, the pillars sliding off the bottom.
//!
//! This module owns geometry and sequencing only. Pixel compositing —
//! decoding `STARTSC`, blitting records, copying rectangles between the
//! hidden surface and the visible page — belongs to the graphical
//! frontend (`u5-bevy`). The terminal harness has no pixel surface and
//! must refuse this screen outright rather than print credit text; see
//! [`crate::intro::require_graphical_acknowledgements_surface`].

use crate::intro::{TITLE_SURFACE_HEIGHT, TITLE_SURFACE_WIDTH};

/// `intro.md §11.1`: the archive the acknowledgement page comes from.
/// `STARTSC` is used by nothing except this path — `#72` retracted the
/// older attributions that gave it to the start/menu screen and sourced
/// the credits from the end-screen family.
pub const ACKNOWLEDGEMENTS_ARCHIVE_STEM: &str = "STARTSC";

/// `intro.md §11.1` record 0: the left ornamental pillar, 16 x 137.
pub const ACK_LEFT_PILLAR_RECORD: u8 = 0;
/// `intro.md §11.1` record 1: the credits page, 288 x 137, with every
/// credit line drawn into the bitmap.
pub const ACK_CREDITS_RECORD: u8 = 1;
/// `intro.md §11.1` record 2: the right ornamental pillar, 16 x 137.
///
/// Records 0 and 2 read as a mirrored pair but record 2 is **not** a
/// horizontal flip of record 0 — `#72` re-checked and found only seven
/// of the 137 row pairs are exact mirrors (one row is identical rather
/// than mirrored), because their dithering is authored separately. It
/// must be decoded from the archive, never synthesised from record 0.
pub const ACK_RIGHT_PILLAR_RECORD: u8 = 2;

/// `intro.md §11.1`: both pillar records are 16 columns wide.
pub const ACK_PILLAR_WIDTH: usize = 16;
/// `intro.md §11.1`: the credits record is 288 columns wide.
pub const ACK_CREDITS_WIDTH: usize = 288;
/// `intro.md §11.1`: all three records are 137 rows tall.
pub const ACK_RECORD_HEIGHT: usize = 137;

/// The 320-by-200 page the whole path draws into.
pub const ACK_SCREEN_WIDTH: usize = TITLE_SURFACE_WIDTH as usize;
pub const ACK_SCREEN_HEIGHT: usize = TITLE_SURFACE_HEIGHT as usize;

/// `intro.md §11.1`: assembled, the three records form one 320-by-137
/// band whose top row is 63, so the finished page occupies rows
/// `63..=199` — the whole screen below the `ULTIMA` logo band.
pub const ACK_BAND_TOP_Y: usize = ACK_SCREEN_HEIGHT - ACK_RECORD_HEIGHT;
/// The band's last row, which is also the last row of the page.
pub const ACK_BAND_BOTTOM_Y: usize = ACK_SCREEN_HEIGHT - 1;
/// `intro.md §11.1`: record 1 sits at `(16, 63)` on the hidden surface.
pub const ACK_CREDITS_ORIGIN_X: usize = ACK_PILLAR_WIDTH;

/// `intro.md §11.2` step 4: the rise phase parks the two pillars side
/// by side on columns `144..=175`.
pub const ACK_LEFT_PILLAR_CENTRE_X: usize = 144;
pub const ACK_RIGHT_PILLAR_CENTRE_X: usize = 160;

/// `intro.md §11.2` step 4: `y` runs from 199 down to 63 inclusive, one
/// pixel per step.
pub const ACK_RISE_STEP_COUNT: usize = ACK_RECORD_HEIGHT;
/// `intro.md §11.2` steps 5 and 8: eighteen steps at an eight-pixel
/// stride, `k = 0, 8, 16 ... 136`.
pub const ACK_BAND_STRIDE: usize = 8;
pub const ACK_PART_STEP_COUNT: usize = 18;
pub const ACK_CLOSE_STEP_COUNT: usize = 18;
/// `intro.md §11.2` step 10: 136 pillar-and-row steps for `y = 63..=198`
/// plus one trailing single-row copy at row 199.
pub const ACK_SINK_PILLAR_STEP_COUNT: usize = ACK_RECORD_HEIGHT - 1;
pub const ACK_SINK_STEP_COUNT: usize = ACK_SINK_PILLAR_STEP_COUNT + 1;

/// `intro.md §11.2` "Wipe cadence": each part and close step ends with a
/// wait of exactly one hardware timer tick (~54.9 ms), so each of those
/// two phases takes about 0.99 s on a host above the boot calibration
/// baseline. The rise and sink phases carry **no wait at all**.
///
/// `timing.md §5.2` (as corrected by `#72`) elides a one-tick request on
/// a host *at or below* the calibration baseline — the earlier "may be
/// skipped on sufficiently fast machines" wording was backwards and is
/// retracted. The engine's host is always above that baseline, so the
/// wait is always performed.
pub const ACK_PACED_STEP_BIOS_TICKS: u8 = 1;
pub const ACK_UNPACED_STEP_BIOS_TICKS: u8 = 0;

/// `intro.md §11.2` step 6: the menu rebuild draws `ULTIMA` record 1 —
/// the first subtitle animation phase, i.e. title-tick frame 0 — at
/// `(16, 65)` on the hidden surface.
pub const ACK_MENU_REBUILD_TITLE_TICK_FRAME: u8 = 0;

/// The five moving phases of `intro.md §11.2`, plus the two bookkeeping
/// states the engine's frame-driven presentation needs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AcknowledgementsPhase {
    /// Step 2: compose record 1 onto the hidden surface. Runs together
    /// with the rise phase because neither carries a wait.
    Compose,
    /// Step 4: the pillars climb the centre of the visible page.
    Rise,
    /// Step 5: two eight-pixel bands expand outward, one BIOS tick each.
    Part,
    /// Step 7: poll the keyboard until any key is returned. `Esc` has no
    /// special meaning here; there is no timeout and no second page.
    AwaitKey,
    /// Step 8: the mirror of the part phase, one BIOS tick each.
    Close,
    /// Step 10: the pillars slide off the bottom edge, unpaced.
    Sink,
    /// Step 12: the path has returned to the menu poll loop.
    Finished,
}

/// One draw the presentation performs. `u5-bevy` executes these; this
/// crate only says what and where.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AcknowledgementsDraw {
    /// `§11.2` step 2: draw `STARTSC` record 1 opaquely onto the
    /// **hidden** surface at `(16, 63)`.
    ComposeCreditsOnHidden,
    /// Draw a `STARTSC` pillar record opaquely onto the **visible**
    /// page at `(x, y)`. Rows past row 199 are clipped away — the rise
    /// and sink phases deliberately place the 137-row record so that it
    /// hangs off the bottom edge. See [`acknowledgements_pillar_visible_rows`].
    Pillar { record: u8, x: usize, y: usize },
    /// Copy the inclusive rectangle `(left, top)..(right, bottom)` from
    /// the hidden surface to the visible page.
    CopyFromHidden {
        left: usize,
        top: usize,
        right: usize,
        bottom: usize,
    },
    /// `§11.2` step 6: rebuild the menu screen on the **hidden** surface
    /// while the credits are still displayed — load `ULTIMA`, issue the
    /// text system's clear control (which blanks the entire hidden page,
    /// because the intro's active text window is the full-screen one),
    /// draw `ULTIMA` record 1 at `(16, 65)`, release the archive, draw
    /// the `§6.1` lower text-window frame, and render the `§6.2` menu
    /// labels with the Acknowledgements row bracketed by the
    /// inverse-video attribute toggle. This is the one place in the path
    /// that touches the text pipeline, and none of it is visible until
    /// the close phase publishes it.
    RebuildMenuOnHidden,
    /// `§11.2` step 11: clear the hidden page a second time and stage
    /// `ULTIMA` records 1..=4 at vertical origins 0, 50, 100 and 150 —
    /// the subtitle animation atlas the `§5` title tick expects to find.
    StageTitleTickAtlasOnHidden,
}

impl AcknowledgementsDraw {
    /// The topmost **visible-page** row this draw writes, or `None` when
    /// the draw lands on the hidden surface.
    ///
    /// `intro.md §11.3`: no pixel above row 63 *of the visible page* is
    /// written at any point, which is why the `ULTIMA` logo on rows
    /// `0..=60` survives. The hidden surface is a different story — it is
    /// written above row 63 twice — so hidden-surface draws report `None`
    /// rather than a row.
    pub fn visible_top_row(self) -> Option<usize> {
        match self {
            Self::ComposeCreditsOnHidden
            | Self::RebuildMenuOnHidden
            | Self::StageTitleTickAtlasOnHidden => None,
            Self::Pillar { y, .. } => Some(y),
            Self::CopyFromHidden { top, .. } => Some(top),
        }
    }

    /// The inclusive visible-page column span this draw publishes, or
    /// `None` for hidden-surface draws.
    pub fn visible_columns(self) -> Option<(usize, usize)> {
        match self {
            Self::ComposeCreditsOnHidden
            | Self::RebuildMenuOnHidden
            | Self::StageTitleTickAtlasOnHidden => None,
            Self::Pillar { x, .. } => Some((x, x + ACK_PILLAR_WIDTH - 1)),
            Self::CopyFromHidden { left, right, .. } => Some((left, right)),
        }
    }
}

/// One presentation step: the draws it performs, in order, and the wait
/// that ends it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AcknowledgementsStep {
    pub phase: AcknowledgementsPhase,
    pub draws: Vec<AcknowledgementsDraw>,
    /// `§11.2` "Wipe cadence": 1 on part and close steps, 0 everywhere
    /// else. There is no other pacing primitive in this path — no
    /// calibrated busy wait, no per-step keyboard probe, no title tick.
    pub wait_bios_ticks: u8,
}

/// How many rows of a 137-row pillar record are on-screen when its top
/// edge is placed at `y` on the 200-row page. The rise and sink phases
/// both start with the record almost entirely below the bottom edge.
pub fn acknowledgements_pillar_visible_rows(y: usize) -> usize {
    ACK_SCREEN_HEIGHT.saturating_sub(y).min(ACK_RECORD_HEIGHT)
}

/// `intro.md §11.2` step 2, bundled with step 3's "select the visible
/// page". Composing the credits page carries no wait.
pub fn acknowledgements_compose_step() -> AcknowledgementsStep {
    AcknowledgementsStep {
        phase: AcknowledgementsPhase::Compose,
        draws: vec![AcknowledgementsDraw::ComposeCreditsOnHidden],
        wait_bios_ticks: ACK_UNPACED_STEP_BIOS_TICKS,
    }
}

/// `intro.md §11.2` step 4. For `y` from 199 down to 63 inclusive, one
/// pixel per step, record 0 at `(144, y)` and record 2 at `(160, y)`.
/// The two pillars climb the centre of the screen from the bottom edge,
/// drawn straight over the still-visible menu window, and come to rest
/// occupying columns `144..=175` at rows `63..=199`.
pub fn acknowledgements_rise_steps() -> Vec<AcknowledgementsStep> {
    (0..ACK_RISE_STEP_COUNT)
        .map(|index| {
            let y = ACK_BAND_BOTTOM_Y - index;
            AcknowledgementsStep {
                phase: AcknowledgementsPhase::Rise,
                draws: vec![
                    AcknowledgementsDraw::Pillar {
                        record: ACK_LEFT_PILLAR_RECORD,
                        x: ACK_LEFT_PILLAR_CENTRE_X,
                        y,
                    },
                    AcknowledgementsDraw::Pillar {
                        record: ACK_RIGHT_PILLAR_RECORD,
                        x: ACK_RIGHT_PILLAR_CENTRE_X,
                        y,
                    },
                ],
                wait_bios_ticks: ACK_UNPACED_STEP_BIOS_TICKS,
            }
        })
        .collect()
}

/// `intro.md §11.2` step 5. For `k = 0, 8, 16 ... 136` (eighteen steps):
/// draw record 0 at `(136 - k, 63)`; copy `(152 - k, 63)..(159 - k, 199)`
/// from the hidden surface; draw record 2 at `(168 + k, 63)`; copy
/// `(160 + k, 63)..(167 + k, 199)`; wait one hardware timer tick.
///
/// The pillars walk outward to `(0, 63)` and `(304, 63)` while two
/// eight-pixel bands expand from the screen centre and publish the
/// credits page column by column. The last step's bands are `16..=23`
/// and `296..=303`, which exactly meet the resting pillars, so the
/// completed band is contiguous.
pub fn acknowledgements_part_steps() -> Vec<AcknowledgementsStep> {
    (0..ACK_PART_STEP_COUNT)
        .map(|index| {
            let k = index * ACK_BAND_STRIDE;
            AcknowledgementsStep {
                phase: AcknowledgementsPhase::Part,
                draws: vec![
                    AcknowledgementsDraw::Pillar {
                        record: ACK_LEFT_PILLAR_RECORD,
                        x: 136 - k,
                        y: ACK_BAND_TOP_Y,
                    },
                    AcknowledgementsDraw::CopyFromHidden {
                        left: 152 - k,
                        top: ACK_BAND_TOP_Y,
                        right: 159 - k,
                        bottom: ACK_BAND_BOTTOM_Y,
                    },
                    AcknowledgementsDraw::Pillar {
                        record: ACK_RIGHT_PILLAR_RECORD,
                        x: 168 + k,
                        y: ACK_BAND_TOP_Y,
                    },
                    AcknowledgementsDraw::CopyFromHidden {
                        left: 160 + k,
                        top: ACK_BAND_TOP_Y,
                        right: 167 + k,
                        bottom: ACK_BAND_BOTTOM_Y,
                    },
                ],
                wait_bios_ticks: ACK_PACED_STEP_BIOS_TICKS,
            }
        })
        .collect()
}

/// `intro.md §11.2` step 6: rebuild the menu screen on the hidden
/// surface while the credits are still displayed. Unpaced, and it sits
/// between the part phase and the keypress wait.
pub fn acknowledgements_menu_rebuild_step() -> AcknowledgementsStep {
    AcknowledgementsStep {
        phase: AcknowledgementsPhase::Part,
        draws: vec![AcknowledgementsDraw::RebuildMenuOnHidden],
        wait_bios_ticks: ACK_UNPACED_STEP_BIOS_TICKS,
    }
}

/// `intro.md §11.2` step 8, the mirror of step 5. For
/// `k = 136, 128, ... 8, 0` (eighteen steps): draw record 0 at
/// `(144 - k, 63)`; copy `(136 - k, 63)..(143 - k, 199)` from the hidden
/// surface; draw record 2 at `(160 + k, 63)`; copy
/// `(176 + k, 63)..(183 + k, 199)`; wait one hardware timer tick.
///
/// Note the band offsets differ from the part phase: the close phase
/// leads with the *outermost* band and the pillars walk back inward to
/// `144..=175`, exactly where the sink phase expects to find them.
pub fn acknowledgements_close_steps() -> Vec<AcknowledgementsStep> {
    (0..ACK_CLOSE_STEP_COUNT)
        .map(|index| {
            let k = (ACK_CLOSE_STEP_COUNT - 1 - index) * ACK_BAND_STRIDE;
            AcknowledgementsStep {
                phase: AcknowledgementsPhase::Close,
                draws: vec![
                    AcknowledgementsDraw::Pillar {
                        record: ACK_LEFT_PILLAR_RECORD,
                        x: 144 - k,
                        y: ACK_BAND_TOP_Y,
                    },
                    AcknowledgementsDraw::CopyFromHidden {
                        left: 136 - k,
                        top: ACK_BAND_TOP_Y,
                        right: 143 - k,
                        bottom: ACK_BAND_BOTTOM_Y,
                    },
                    AcknowledgementsDraw::Pillar {
                        record: ACK_RIGHT_PILLAR_RECORD,
                        x: 160 + k,
                        y: ACK_BAND_TOP_Y,
                    },
                    AcknowledgementsDraw::CopyFromHidden {
                        left: 176 + k,
                        top: ACK_BAND_TOP_Y,
                        right: 183 + k,
                        bottom: ACK_BAND_BOTTOM_Y,
                    },
                ],
                wait_bios_ticks: ACK_PACED_STEP_BIOS_TICKS,
            }
        })
        .collect()
}

/// `intro.md §11.2` step 10, with step 11 appended to the final step.
///
/// For `y` from 63 to 198 inclusive, one pixel per step, draw record 0
/// at `(144, y + 1)` and record 2 at `(160, y + 1)`, then copy the
/// single-row inclusive rectangle `(144, y)..(175, y)` from the hidden
/// surface. Finish with one more single-row copy of
/// `(144, 199)..(175, 199)`. The two pillars slide off the bottom of the
/// screen and the last centre column of the menu screen is published
/// behind them. Step 11 then restages the subtitle animation atlas on
/// the hidden surface; it carries no wait either, so it rides along on
/// the final sink step.
pub fn acknowledgements_sink_steps() -> Vec<AcknowledgementsStep> {
    let mut steps: Vec<AcknowledgementsStep> = (0..ACK_SINK_PILLAR_STEP_COUNT)
        .map(|index| {
            let y = ACK_BAND_TOP_Y + index;
            AcknowledgementsStep {
                phase: AcknowledgementsPhase::Sink,
                draws: vec![
                    AcknowledgementsDraw::Pillar {
                        record: ACK_LEFT_PILLAR_RECORD,
                        x: ACK_LEFT_PILLAR_CENTRE_X,
                        y: y + 1,
                    },
                    AcknowledgementsDraw::Pillar {
                        record: ACK_RIGHT_PILLAR_RECORD,
                        x: ACK_RIGHT_PILLAR_CENTRE_X,
                        y: y + 1,
                    },
                    AcknowledgementsDraw::CopyFromHidden {
                        left: ACK_LEFT_PILLAR_CENTRE_X,
                        top: y,
                        right: ACK_RIGHT_PILLAR_CENTRE_X + ACK_PILLAR_WIDTH - 1,
                        bottom: y,
                    },
                ],
                wait_bios_ticks: ACK_UNPACED_STEP_BIOS_TICKS,
            }
        })
        .collect();
    steps.push(AcknowledgementsStep {
        phase: AcknowledgementsPhase::Sink,
        draws: vec![
            AcknowledgementsDraw::CopyFromHidden {
                left: ACK_LEFT_PILLAR_CENTRE_X,
                top: ACK_BAND_BOTTOM_Y,
                right: ACK_RIGHT_PILLAR_CENTRE_X + ACK_PILLAR_WIDTH - 1,
                bottom: ACK_BAND_BOTTOM_Y,
            },
            AcknowledgementsDraw::StageTitleTickAtlasOnHidden,
        ],
        wait_bios_ticks: ACK_UNPACED_STEP_BIOS_TICKS,
    });
    steps
}

/// Which draw last published a given screen column during a phase.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AcknowledgementsColumnOwner {
    /// The left ornamental pillar came to rest here.
    LeftPillar,
    /// The right ornamental pillar came to rest here.
    RightPillar,
    /// An eight-pixel band copied from the hidden surface on this step.
    Band { step: usize },
}

/// `intro.md §11.2` "No pre-clear": nothing clears visible rows
/// `63..=199` before the credits appear, so the coverage has to be exact
/// instead. Replays a phase's draws in order and reports, per screen
/// column, which draw published it last.
///
/// The invariant the part phase must satisfy is that every column of the
/// 320-by-137 band is published exactly once: the left bands sweep
/// columns `16..=159`, the right bands `160..=303`, and the two pillars
/// come to rest on `0..=15` and `304..=319`. It is the property most
/// likely to catch an off-by-one in the band arithmetic, so it is
/// exposed here rather than buried in a test.
pub fn acknowledgements_final_column_owners(
    steps: &[AcknowledgementsStep],
) -> [Option<AcknowledgementsColumnOwner>; ACK_SCREEN_WIDTH] {
    let mut owners = [None; ACK_SCREEN_WIDTH];
    for (step_index, step) in steps.iter().enumerate() {
        for draw in &step.draws {
            let Some((left, right)) = draw.visible_columns() else {
                continue;
            };
            let owner = match draw {
                AcknowledgementsDraw::Pillar { record, .. } => {
                    if *record == ACK_LEFT_PILLAR_RECORD {
                        AcknowledgementsColumnOwner::LeftPillar
                    } else {
                        AcknowledgementsColumnOwner::RightPillar
                    }
                }
                _ => AcknowledgementsColumnOwner::Band { step: step_index },
            };
            assert!(
                right < ACK_SCREEN_WIDTH,
                "acknowledgements draw publishes column {right}, past the 320-column page"
            );
            for column in owners.iter_mut().take(right + 1).skip(left) {
                *column = Some(owner);
            }
        }
    }
    owners
}

/// Frame-driven cursor over the `§11.2` step list.
///
/// The engine's graphical intro runs on a DOS BIOS user-tick pump
/// (~18.2 Hz), which is exactly the cadence the part and close phases
/// ask for. The rise and sink phases carry no wait, so they run as one
/// unpaced burst — the same treatment `cleak/u5-spec#53` gave the story
/// reveal, where no wall-clock duration is published and there is
/// therefore no rate to spread the steps over frames with. Inventing a
/// per-step rise cadence would be inventing a timing the spec does not
/// publish.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AcknowledgementsPresentation {
    phase: AcknowledgementsPhase,
    step: usize,
    key_queued: bool,
}

impl Default for AcknowledgementsPresentation {
    fn default() -> Self {
        Self::new()
    }
}

impl AcknowledgementsPresentation {
    pub fn new() -> Self {
        Self {
            phase: AcknowledgementsPhase::Compose,
            step: 0,
            key_queued: false,
        }
    }

    pub fn phase(&self) -> AcknowledgementsPhase {
        self.phase
    }

    /// Index within the current phase's step list.
    pub fn step_index(&self) -> usize {
        self.step
    }

    pub fn is_finished(&self) -> bool {
        self.phase == AcknowledgementsPhase::Finished
    }

    /// True while `§11.2` step 7's any-key poll is the only thing left
    /// to do.
    pub fn awaiting_key(&self) -> bool {
        self.phase == AcknowledgementsPhase::AwaitKey
    }

    /// `§11.2` step 7: any key advances, and `Esc` has no special
    /// meaning here. Keys struck earlier land in the BIOS type-ahead
    /// buffer and step 7's poll returns one immediately, so a key
    /// pressed during the rise or part phases is remembered rather than
    /// dropped. Keys arriving during the close and sink phases are
    /// discarded by step 12's type-ahead flush.
    pub fn queue_key(&mut self) {
        match self.phase {
            AcknowledgementsPhase::Compose
            | AcknowledgementsPhase::Rise
            | AcknowledgementsPhase::Part => self.key_queued = true,
            AcknowledgementsPhase::AwaitKey => {
                self.key_queued = false;
                self.phase = AcknowledgementsPhase::Close;
                self.step = 0;
            }
            AcknowledgementsPhase::Close
            | AcknowledgementsPhase::Sink
            | AcknowledgementsPhase::Finished => {}
        }
    }

    /// Produce the draws that run before the next wait, advancing the
    /// cursor past them.
    ///
    /// Call once when the panel opens — that runs the unpaced compose
    /// and rise burst, so the first frame the player sees already has
    /// the pillars at rest — then once per BIOS user-tick. Returns an
    /// empty list while the keypress wait is open and once the path has
    /// finished.
    pub fn advance(&mut self) -> Vec<AcknowledgementsStep> {
        match self.phase {
            AcknowledgementsPhase::Compose => {
                let mut steps = vec![acknowledgements_compose_step()];
                steps.extend(acknowledgements_rise_steps());
                self.phase = AcknowledgementsPhase::Part;
                self.step = 0;
                steps
            }
            AcknowledgementsPhase::Rise => {
                // The rise phase is never left pending across a tick;
                // it is emitted with the compose step above.
                unreachable!("acknowledgements rise phase runs inside the compose burst")
            }
            AcknowledgementsPhase::Part => {
                let part = acknowledgements_part_steps();
                let mut steps = vec![part[self.step].clone()];
                self.step += 1;
                if self.step >= ACK_PART_STEP_COUNT {
                    // Step 6 rebuilds the menu on the hidden surface
                    // while the credits are still displayed, between the
                    // part phase and the keypress wait.
                    steps.push(acknowledgements_menu_rebuild_step());
                    self.step = 0;
                    if self.key_queued {
                        self.key_queued = false;
                        self.phase = AcknowledgementsPhase::Close;
                    } else {
                        self.phase = AcknowledgementsPhase::AwaitKey;
                    }
                }
                steps
            }
            AcknowledgementsPhase::AwaitKey | AcknowledgementsPhase::Finished => Vec::new(),
            AcknowledgementsPhase::Close => {
                let close = acknowledgements_close_steps();
                let steps = vec![close[self.step].clone()];
                self.step += 1;
                if self.step >= ACK_CLOSE_STEP_COUNT {
                    self.step = 0;
                    self.phase = AcknowledgementsPhase::Sink;
                }
                steps
            }
            AcknowledgementsPhase::Sink => {
                let steps = acknowledgements_sink_steps();
                self.step = 0;
                self.phase = AcknowledgementsPhase::Finished;
                steps
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `intro.md §11.1` / `cleak/u5-spec#72`: three records, 16 + 288 +
    /// 16 by 137, assembled into one 320-by-137 band whose top row is 63.
    #[test]
    fn acknowledgements_band_geometry_matches_the_published_records() {
        assert_eq!(ACKNOWLEDGEMENTS_ARCHIVE_STEM, "STARTSC");
        assert_eq!(ACK_PILLAR_WIDTH * 2 + ACK_CREDITS_WIDTH, ACK_SCREEN_WIDTH);
        assert_eq!(ACK_RECORD_HEIGHT, 137);
        assert_eq!(ACK_BAND_TOP_Y, 63);
        assert_eq!(ACK_BAND_BOTTOM_Y, 199);
        assert_eq!(ACK_CREDITS_ORIGIN_X, 16);
    }

    /// `intro.md §11.2`: 137 rise steps, 18 part steps, 18 close steps,
    /// and 136 + 1 sink steps.
    #[test]
    fn acknowledgements_phase_step_counts_match_the_published_sequence() {
        assert_eq!(acknowledgements_rise_steps().len(), 137);
        assert_eq!(acknowledgements_part_steps().len(), 18);
        assert_eq!(acknowledgements_close_steps().len(), 18);
        assert_eq!(acknowledgements_sink_steps().len(), 137);
        assert_eq!(ACK_SINK_PILLAR_STEP_COUNT, 136);
    }

    /// `intro.md §11.2` "Wipe cadence": only the part and close phases
    /// are paced, one hardware timer tick per step. The rise and sink
    /// phases carry no wait at all.
    #[test]
    fn only_part_and_close_steps_wait_one_bios_tick() {
        for step in acknowledgements_part_steps() {
            assert_eq!(step.wait_bios_ticks, 1, "every part step waits one tick");
        }
        for step in acknowledgements_close_steps() {
            assert_eq!(step.wait_bios_ticks, 1, "every close step waits one tick");
        }
        for step in acknowledgements_rise_steps() {
            assert_eq!(step.wait_bios_ticks, 0, "the rise phase carries no wait");
        }
        for step in acknowledgements_sink_steps() {
            assert_eq!(step.wait_bios_ticks, 0, "the sink phase carries no wait");
        }
        assert_eq!(acknowledgements_compose_step().wait_bios_ticks, 0);
        assert_eq!(acknowledgements_menu_rebuild_step().wait_bios_ticks, 0);
        // Each paced phase is eighteen ticks, ~0.99 s at ~54.9 ms.
        let paced: u32 = acknowledgements_part_steps()
            .iter()
            .map(|step| u32::from(step.wait_bios_ticks))
            .sum();
        assert_eq!(paced, 18);
    }

    /// `intro.md §11.2` "No pre-clear": nothing clears visible rows
    /// `63..=199` before the credits appear, so the part phase's coverage
    /// must be exact — its left bands sweep columns `16..=159`, its right
    /// bands `160..=303`, and the two pillars come to rest on `0..=15`
    /// and `304..=319`, publishing every column exactly once.
    #[test]
    fn part_phase_publishes_every_column_exactly_once() {
        let steps = acknowledgements_part_steps();
        let owners = acknowledgements_final_column_owners(&steps);

        for (column, owner) in owners.iter().enumerate() {
            let owner = owner.unwrap_or_else(|| {
                panic!("part phase left column {column} of the band unpublished")
            });
            let expected = match column {
                0..=15 => AcknowledgementsColumnOwner::LeftPillar,
                304..=319 => AcknowledgementsColumnOwner::RightPillar,
                _ => {
                    assert!(
                        matches!(owner, AcknowledgementsColumnOwner::Band { .. }),
                        "column {column} must be published by a band copy, got {owner:?}"
                    );
                    continue;
                }
            };
            assert_eq!(owner, expected, "column {column}");
        }

        // The band copies themselves must be pairwise disjoint and cover
        // exactly 16..=303 - the off-by-one guard on the arithmetic.
        let mut band_hits = [0u32; ACK_SCREEN_WIDTH];
        for step in &steps {
            for draw in &step.draws {
                if let AcknowledgementsDraw::CopyFromHidden { left, right, .. } = draw {
                    for hit in band_hits.iter_mut().take(right + 1).skip(*left) {
                        *hit += 1;
                    }
                }
            }
        }
        for (column, hits) in band_hits.iter().enumerate() {
            let expected = u32::from((16..=303).contains(&column));
            assert_eq!(*hits, expected, "band copies over column {column}");
        }
    }

    /// The close phase publishes the rebuilt menu from the outside
    /// inward and must satisfy the same exact-coverage property, with
    /// the pillars coming to rest on `144..=175` where the sink phase
    /// picks them up.
    #[test]
    fn close_phase_publishes_every_column_exactly_once() {
        let steps = acknowledgements_close_steps();
        let owners = acknowledgements_final_column_owners(&steps);

        for (column, owner) in owners.iter().enumerate() {
            let owner = owner.unwrap_or_else(|| {
                panic!("close phase left column {column} of the band unpublished")
            });
            match column {
                144..=159 => assert_eq!(owner, AcknowledgementsColumnOwner::LeftPillar, "{column}"),
                160..=175 => {
                    assert_eq!(owner, AcknowledgementsColumnOwner::RightPillar, "{column}")
                }
                _ => assert!(
                    matches!(owner, AcknowledgementsColumnOwner::Band { .. }),
                    "column {column} must be published by a band copy, got {owner:?}"
                ),
            }
        }

        let mut band_hits = [0u32; ACK_SCREEN_WIDTH];
        for step in &steps {
            for draw in &step.draws {
                if let AcknowledgementsDraw::CopyFromHidden { left, right, .. } = draw {
                    for hit in band_hits.iter_mut().take(right + 1).skip(*left) {
                        *hit += 1;
                    }
                }
            }
        }
        for (column, hits) in band_hits.iter().enumerate() {
            let expected = u32::from(!(144..=175).contains(&column));
            assert_eq!(*hits, expected, "band copies over column {column}");
        }
    }

    /// `intro.md §11.3`: no pixel above row 63 **of the visible page** is
    /// written at any point, which is why the `ULTIMA` logo on rows
    /// `0..=60` survives. Hidden-surface draws are exempt — the hidden
    /// page is written above row 63 twice, by the menu rebuild and by the
    /// subtitle atlas restage.
    #[test]
    fn no_visible_pixel_above_row_63_is_ever_written() {
        let mut all = vec![acknowledgements_compose_step()];
        all.extend(acknowledgements_rise_steps());
        all.extend(acknowledgements_part_steps());
        all.push(acknowledgements_menu_rebuild_step());
        all.extend(acknowledgements_close_steps());
        all.extend(acknowledgements_sink_steps());

        let mut hidden_draws = 0;
        for step in &all {
            for draw in &step.draws {
                match draw.visible_top_row() {
                    Some(top) => assert!(
                        top >= ACK_BAND_TOP_Y,
                        "{draw:?} writes visible row {top}, above the §11.3 floor of 63"
                    ),
                    None => hidden_draws += 1,
                }
            }
        }
        // Compose, menu rebuild, subtitle atlas restage.
        assert_eq!(hidden_draws, 3);
    }

    /// `intro.md §11.2` step 4 and step 10 both hang the 137-row pillar
    /// record off the bottom edge of the 200-row page.
    #[test]
    fn pillar_records_clip_against_the_bottom_edge() {
        assert_eq!(acknowledgements_pillar_visible_rows(ACK_BAND_BOTTOM_Y), 1);
        assert_eq!(acknowledgements_pillar_visible_rows(ACK_BAND_TOP_Y), 137);
        assert_eq!(acknowledgements_pillar_visible_rows(ACK_SCREEN_HEIGHT), 0);
        assert_eq!(acknowledgements_pillar_visible_rows(150), 50);
    }

    /// `intro.md §11.2` step 8's band offsets differ from step 5's: the
    /// close phase must leave the pillars on `144..=175`, exactly where
    /// step 10 redraws them.
    #[test]
    fn close_phase_parks_the_pillars_where_the_sink_phase_starts() {
        let close = acknowledgements_close_steps();
        let last = close.last().expect("close phase has steps");
        assert_eq!(
            last.draws[0],
            AcknowledgementsDraw::Pillar {
                record: ACK_LEFT_PILLAR_RECORD,
                x: ACK_LEFT_PILLAR_CENTRE_X,
                y: ACK_BAND_TOP_Y,
            }
        );
        assert_eq!(
            last.draws[2],
            AcknowledgementsDraw::Pillar {
                record: ACK_RIGHT_PILLAR_RECORD,
                x: ACK_RIGHT_PILLAR_CENTRE_X,
                y: ACK_BAND_TOP_Y,
            }
        );
        let first_sink = &acknowledgements_sink_steps()[0];
        assert_eq!(
            first_sink.draws[0],
            AcknowledgementsDraw::Pillar {
                record: ACK_LEFT_PILLAR_RECORD,
                x: ACK_LEFT_PILLAR_CENTRE_X,
                y: ACK_BAND_TOP_Y + 1,
            }
        );
    }

    /// `intro.md §11.2` step 10 publishes rows `63..=199` of the centre
    /// columns one row at a time, ending with the trailing row-199 copy.
    #[test]
    fn sink_phase_publishes_every_centre_row_exactly_once() {
        let mut rows = [0u32; ACK_SCREEN_HEIGHT];
        for step in acknowledgements_sink_steps() {
            for draw in step.draws {
                if let AcknowledgementsDraw::CopyFromHidden {
                    left,
                    top,
                    right,
                    bottom,
                } = draw
                {
                    assert_eq!((left, right), (144, 175), "sink copies the centre columns");
                    assert_eq!(top, bottom, "sink copies exactly one row per step");
                    rows[top] += 1;
                }
            }
        }
        for (row, hits) in rows.iter().enumerate() {
            let expected = u32::from((ACK_BAND_TOP_Y..=ACK_BAND_BOTTOM_Y).contains(&row));
            assert_eq!(*hits, expected, "sink copies over row {row}");
        }
    }

    /// `intro.md §11.2` steps 2..12 driven the way the graphical intro
    /// drives them: one unpaced burst at entry, eighteen paced part
    /// ticks, the keypress wait, eighteen paced close ticks, then one
    /// unpaced sink burst.
    #[test]
    fn presentation_drives_the_published_phase_order() {
        let mut presentation = AcknowledgementsPresentation::new();

        let opening = presentation.advance();
        assert_eq!(opening.len(), 1 + 137, "compose plus the whole rise phase");
        assert_eq!(opening[0].phase, AcknowledgementsPhase::Compose);
        assert_eq!(opening[1].phase, AcknowledgementsPhase::Rise);
        assert_eq!(presentation.phase(), AcknowledgementsPhase::Part);

        for tick in 0..ACK_PART_STEP_COUNT {
            let steps = presentation.advance();
            if tick + 1 < ACK_PART_STEP_COUNT {
                assert_eq!(steps.len(), 1, "part tick {tick}");
            } else {
                // The last part tick carries the step 6 menu rebuild.
                assert_eq!(steps.len(), 2, "final part tick carries the menu rebuild");
                assert_eq!(
                    steps[1].draws,
                    vec![AcknowledgementsDraw::RebuildMenuOnHidden]
                );
            }
        }
        assert!(presentation.awaiting_key());

        // Step 7: no timeout and no auto-advance - ticking does nothing.
        assert!(presentation.advance().is_empty());
        assert!(presentation.awaiting_key());

        presentation.queue_key();
        assert_eq!(presentation.phase(), AcknowledgementsPhase::Close);
        for tick in 0..ACK_CLOSE_STEP_COUNT {
            assert_eq!(presentation.advance().len(), 1, "close tick {tick}");
        }
        assert_eq!(presentation.phase(), AcknowledgementsPhase::Sink);

        let sink = presentation.advance();
        assert_eq!(sink.len(), 137);
        assert!(presentation.is_finished());
        assert!(presentation.advance().is_empty());
    }

    /// `intro.md §11.2` step 7: a key struck while the credits are still
    /// parting lands in the type-ahead buffer, and the poll returns it
    /// immediately, so the wait never opens.
    #[test]
    fn key_struck_during_the_part_phase_is_consumed_by_the_step_7_poll() {
        let mut presentation = AcknowledgementsPresentation::new();
        presentation.advance();
        presentation.advance();
        presentation.queue_key();
        for _ in 1..ACK_PART_STEP_COUNT {
            presentation.advance();
        }
        assert!(
            !presentation.awaiting_key(),
            "the queued key satisfies step 7's poll at once"
        );
        assert_eq!(presentation.phase(), AcknowledgementsPhase::Close);
    }
}

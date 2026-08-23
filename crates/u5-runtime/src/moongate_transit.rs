//! `overworld.md §9.2` (spec HEAD `c00bf63`): the blocking transit
//! transition the overworld live-gate entry hook plays before the party
//! is relocated.
//!
//! The hook reads the party's current live terrain cell and returns
//! immediately unless that cell is `0xDC`. **That terrain test is the
//! only precondition** - nothing about daylight, moon phase, transport,
//! surface versus Underworld plane or party composition gates it, and it
//! is confined to the overworld only because it is the overworld loop
//! that runs it.
//!
//! On `0xDC` the hook runs a **blocking** transition to completion before
//! the party is relocated and before any key is read. It is not driven by
//! the per-turn tile animator and it cannot be skipped: the abort poll
//! some other presentation effects offer is disabled in overworld scenes.
//! The whole sequence plays at the **gate cell**, which is the party's own
//! cell and therefore the centre of the eleven-by-eleven view; nothing is
//! played at the destination cell.
//!
//! The published sequence, in order:
//!
//! 1. One world-tick pause, then a short PC-speaker sweep from the shared
//!    parameter-sweep sound helper. We implement no audio, so only the
//!    pause is modelled here. (An earlier revision named the shrine effect
//!    as another user of that helper; `cleak/u5-spec#85` **withdraws** that
//!    comparison as a mis-identified routine. Nothing about the timing,
//!    frame counts, tile ids or the counter's lifetime changed with it, and
//!    no sound relationship is built from it here.)
//! 2. **Stage A, the party is swallowed.** The party sprite is switched to
//!    tile `0x116` and the party's view cell is dissolved into the moon-gate
//!    tile pixel by pixel: the cell is first cleared to colour zero, then
//!    **255** of its 256 pixels are plotted in a fixed pseudo-random order,
//!    one pixel per step. The count is 255, not 256 - the shuffle that
//!    orders the pixels never reaches one of them, so a single pixel of the
//!    cell is left at colour zero when the stage ends; step 4 repaints it a
//!    moment later. The stage is paced by a world tick every **eight** steps
//!    rather than by a fixed wait, so it also advances ambient animation
//!    while it runs.
//! 3. **Stage B, the gate closes.** The party sprite is suppressed entirely
//!    and the shared presence counter is driven from `15` down to `1`, one
//!    phase per step, with a wait of **two BIOS timer ticks** between
//!    phases. Each phase draws the composed frame of `§9.1` at the gate
//!    cell, so the gate sinks back into the ground with the party already
//!    gone. The countdown ends with the counter at zero.
//! 4. The gate's live cell is rewritten to terrain `5`, the viewport is
//!    marked dirty, and the cell is repainted.
//!
//! "The frame counts are `15` for stage B and `256` dispatch steps for
//! stage A; both are exact, and neither is a duration an implementation may
//! retune without changing observable behaviour."
//!
//! **On the two `0x116` uses.** `§9.1` composes each transition frame into
//! the dedicated scratch tile `0x116`, saving and restoring the slot around
//! every composition, and says explicitly that "the same id doubles as the
//! party-vanishing sprite in Section 9.2". Both uses meet here for the
//! first time: stage A *reads* the slot as the party sprite while stage B
//! *writes* composed frames into it. Every write in this module therefore
//! goes through [`with_moongate_phase_scratch_tile`], so the shipped
//! artwork of `0x116` survives the transit intact.

use std::io;

use crate::{
    DissolveVisitOrder, MOONGATE_PHASE_SCRATCH_TILE, NATURAL_MOONGATE_RESTORED_TERRAIN_TILE,
    TILE_ATLAS_TILE_PIXELS, moongate_phase_gate_tile, with_moongate_phase_scratch_tile,
};

/// `overworld.md §9.2`: the transition is **blocking** and runs to
/// completion before the party is relocated and before any key is read.
pub const MOONGATE_TRANSIT_IS_BLOCKING: bool = true;

/// `overworld.md §9.2`: "it cannot be skipped by the player - the abort
/// poll that some other presentation effects offer is disabled in
/// overworld scenes."
pub const MOONGATE_TRANSIT_ABORT_POLL_ENABLED: bool = false;

/// `overworld.md §9.2` step 1: one world-tick pause opens the sequence.
/// The PC-speaker sweep that follows it is not modelled; we implement no
/// audio.
pub const MOONGATE_TRANSIT_OPENING_WORLD_TICKS: u8 = 1;

/// `overworld.md §9.2` stage A: "`256` dispatch steps for stage A ...
/// exact". One step clears the cell to colour zero; the remaining
/// [`MOONGATE_TRANSIT_STAGE_A_PLOTTED_PIXELS`] plot one pixel each.
pub const MOONGATE_TRANSIT_STAGE_A_DISPATCH_STEPS: usize = 256;

/// `overworld.md §9.2` stage A: "**255** of its 256 pixels are plotted in
/// a fixed pseudo-random order, one pixel per step. The count is 255, not
/// 256 - the shuffle that orders the pixels never reaches one of them."
pub const MOONGATE_TRANSIT_STAGE_A_PLOTTED_PIXELS: usize = 255;

/// `overworld.md §9.2` stage A: "paced by a world tick every eight steps
/// rather than by a fixed wait, so it also advances ambient animation
/// while it runs."
pub const MOONGATE_TRANSIT_STAGE_A_WORLD_TICK_EVERY: usize = 8;

/// The world ticks stage A spends, which follows from the two published
/// numbers above: 256 dispatch steps, one tick every eight.
pub const MOONGATE_TRANSIT_STAGE_A_WORLD_TICKS: usize =
    MOONGATE_TRANSIT_STAGE_A_DISPATCH_STEPS / MOONGATE_TRANSIT_STAGE_A_WORLD_TICK_EVERY;

/// `overworld.md §9.2` stage A: "the cell is first cleared to colour
/// zero". The one pixel the shuffle never reaches keeps this value until
/// step 4 repaints the cell.
pub const MOONGATE_TRANSIT_CLEAR_COLOUR: u8 = 0;

/// `overworld.md §9.2` stage A: "The party sprite is switched to tile
/// `0x116`" - the same id `§9.1` uses as its composition scratch, which is
/// exactly why that composition saves and restores the slot.
pub const MOONGATE_TRANSIT_PARTY_VANISH_TILE: usize = MOONGATE_PHASE_SCRATCH_TILE;

/// `overworld.md §9.2` stage B: "The frame counts are `15` for stage B".
pub const MOONGATE_TRANSIT_STAGE_B_STEPS: usize = 15;

/// `overworld.md §9.2` stage B: the counter is driven "from `15` down to
/// `1`, one phase per step".
pub const MOONGATE_TRANSIT_STAGE_B_FIRST_PHASE: u8 = 15;

/// `overworld.md §9.2` stage B: the last phase drawn before the counter
/// lands on zero.
pub const MOONGATE_TRANSIT_STAGE_B_LAST_PHASE: u8 = 1;

/// `overworld.md §9.2` stage B: "a wait of **two BIOS timer ticks**
/// between phases - roughly 110 ms per phase at the standard 18.2 Hz
/// tick, and about 1.65 seconds for the stage."
pub const MOONGATE_TRANSIT_STAGE_B_STEP_BIOS_TICKS: u8 = 2;

/// `overworld.md §9.2` stage B: "The countdown ends with the counter at
/// zero."
///
/// `§9.1` calls out the consequence and tells implementations not to
/// design around it: because the presence counter is shared, a gate that
/// was mid-rise elsewhere in view is driven to zero by an unrelated
/// party's transit and rises again from zero on subsequent turns. "That is
/// the original's behaviour, not a defect to design around."
pub const MOONGATE_TRANSIT_END_COUNTER: u8 = 0;

/// `overworld.md §9.2` step 4: "The gate's live cell is rewritten to
/// terrain `5`".
pub const MOONGATE_TRANSIT_CLEARED_TERRAIN: u8 = NATURAL_MOONGATE_RESTORED_TERRAIN_TILE;

/// `overworld.md §9.2`: how the party is drawn during one transit step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoongateTransitPartySprite {
    /// Before the sequence proper, the party is still its ordinary sprite.
    Party,
    /// Stage A: "The party sprite is switched to tile `0x116`."
    Tile(usize),
    /// Stage B: "the party sprite is suppressed entirely".
    Suppressed,
}

/// `overworld.md §9.2`: one dispatch step of the blocking transit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoongateTransitStep {
    /// Step 1: one world-tick pause, then the PC-speaker sweep. No audio
    /// is modelled.
    OpeningPause { world_ticks: u8 },
    /// Stage A's first dispatch step: the cell is cleared to colour zero.
    StageAClearCell { colour: u8 },
    /// Stage A: plot one pixel of the cell from the moon-gate tile.
    /// `dispatch_index` is the step's index within stage A's 256.
    StageAPlotPixel {
        dispatch_index: usize,
        pixel: usize,
        world_tick: bool,
    },
    /// Stage B: draw the `§9.1` composed frame at `phase`, then wait.
    StageBPhase { phase: u8, wait_bios_ticks: u8 },
    /// Step 4: rewrite the live cell to terrain `5` and repaint it. The
    /// counter is already at zero by here.
    ClearGateCell { terrain: u8 },
}

impl MoongateTransitStep {
    /// `overworld.md §9.2`: how the party is drawn while this step is on
    /// screen - tile `0x116` through stage A, suppressed through stage B.
    pub const fn party_sprite(self) -> MoongateTransitPartySprite {
        match self {
            Self::OpeningPause { .. } => MoongateTransitPartySprite::Party,
            Self::StageAClearCell { .. } | Self::StageAPlotPixel { .. } => {
                MoongateTransitPartySprite::Tile(MOONGATE_TRANSIT_PARTY_VANISH_TILE)
            }
            Self::StageBPhase { .. } | Self::ClearGateCell { .. } => {
                MoongateTransitPartySprite::Suppressed
            }
        }
    }

    /// World ticks this step spends. Stage A is paced by a world tick
    /// every eight steps rather than by a fixed wait; stage B is paced by
    /// BIOS ticks instead and spends none.
    pub const fn world_ticks(self) -> u8 {
        match self {
            Self::OpeningPause { world_ticks } => world_ticks,
            Self::StageAPlotPixel { world_tick, .. } => world_tick as u8,
            Self::StageAClearCell { .. }
            | Self::StageBPhase { .. }
            | Self::ClearGateCell { .. } => 0,
        }
    }

    /// BIOS timer ticks this step waits. Only stage B carries a wait, and
    /// it is two ticks per phase.
    pub const fn wait_bios_ticks(self) -> u8 {
        match self {
            Self::StageBPhase {
                wait_bios_ticks, ..
            } => wait_bios_ticks,
            _ => 0,
        }
    }
}

/// `overworld.md §9.2` stage A: the fixed pseudo-random order the cell's
/// pixels are plotted in.
///
/// This is **the engine's existing dissolve primitive**, not a second one:
/// [`DissolveVisitOrder`] is the shared Galois-LFSR visit order behind
/// every rectangle dissolve here, and asking it for 255 indices is exactly
/// the shape `§9.2` publishes. An eight-bit maximal-length register visits
/// its 255 nonzero states once each, which is why the original plots 255
/// of 256 pixels and leaves one at colour zero - "the shuffle that orders
/// the pixels never reaches one of them". Requesting 256 would widen the
/// register to nine bits and plot all 256, which is the off-by-one `§9.2`
/// warns about.
///
/// `§9.2` publishes that the order is fixed and pseudo-random and that it
/// misses exactly one pixel; it does not publish the tap inventory, so
/// *which* pixel is missed follows from the shared primitive rather than
/// from spec text. `display-driver-abi.md §9.6` takes the same position
/// for the driver-level dissolve.
pub fn moongate_transit_stage_a_pixel_order() -> io::Result<Vec<usize>> {
    let mut order = DissolveVisitOrder::new(MOONGATE_TRANSIT_STAGE_A_PLOTTED_PIXELS)?;
    let mut pixels = Vec::with_capacity(MOONGATE_TRANSIT_STAGE_A_PLOTTED_PIXELS);
    while let Some(index) = order.next_index() {
        pixels.push(index);
    }
    if pixels.len() != MOONGATE_TRANSIT_STAGE_A_PLOTTED_PIXELS {
        return Err(io::Error::other(format!(
            "overworld.md §9.2 stage A plots exactly {} of the cell's {TILE_ATLAS_TILE_PIXELS} \
             pixels; the shared dissolve order yielded {}",
            MOONGATE_TRANSIT_STAGE_A_PLOTTED_PIXELS,
            pixels.len()
        )));
    }
    Ok(pixels)
}

/// `overworld.md §9.2`: the whole blocking transit as an ordered dispatch
/// script - step 1, stage A's 256 dispatch steps, stage B's 15 phases, and
/// step 4's cell rewrite.
pub fn moongate_transit_steps() -> io::Result<Vec<MoongateTransitStep>> {
    let order = moongate_transit_stage_a_pixel_order()?;
    let mut steps = Vec::with_capacity(
        // opening + stage A + stage B + the cell rewrite
        1 + MOONGATE_TRANSIT_STAGE_A_DISPATCH_STEPS + MOONGATE_TRANSIT_STAGE_B_STEPS + 1,
    );
    steps.push(MoongateTransitStep::OpeningPause {
        world_ticks: MOONGATE_TRANSIT_OPENING_WORLD_TICKS,
    });

    // Stage A. Dispatch step 0 clears the cell to colour zero; steps
    // 1..=255 plot one pixel each, which is how 256 exact dispatch steps
    // and 255 plotted pixels are the same stage.
    steps.push(MoongateTransitStep::StageAClearCell {
        colour: MOONGATE_TRANSIT_CLEAR_COLOUR,
    });
    for (plotted, pixel) in order.into_iter().enumerate() {
        let dispatch_index = plotted + 1;
        steps.push(MoongateTransitStep::StageAPlotPixel {
            dispatch_index,
            pixel,
            // A world tick closes each group of eight dispatch steps,
            // counting the clear as the first of the first group: steps
            // 7, 15, ... 255, so exactly 32 ticks across the stage's 256.
            // The pace - one tick per eight steps - is the published
            // part; which step of the eight carries it is not separately
            // observable.
            world_tick: dispatch_index % MOONGATE_TRANSIT_STAGE_A_WORLD_TICK_EVERY
                == MOONGATE_TRANSIT_STAGE_A_WORLD_TICK_EVERY - 1,
        });
    }

    // Stage B: 15 down to 1, one phase per step, two BIOS ticks apiece.
    for phase in (MOONGATE_TRANSIT_STAGE_B_LAST_PHASE..=MOONGATE_TRANSIT_STAGE_B_FIRST_PHASE).rev()
    {
        steps.push(MoongateTransitStep::StageBPhase {
            phase,
            wait_bios_ticks: MOONGATE_TRANSIT_STAGE_B_STEP_BIOS_TICKS,
        });
    }

    steps.push(MoongateTransitStep::ClearGateCell {
        terrain: MOONGATE_TRANSIT_CLEARED_TERRAIN,
    });
    Ok(steps)
}

/// What one run of the transit spent, for callers and tests that need to
/// see the blocking sequence actually ran to completion.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MoongateTransitPlayback {
    /// Stage A's dispatch steps - `256` when the stage ran whole.
    pub stage_a_dispatch_steps: u16,
    /// Stage A's plotted pixels - `255`, one short of the cell.
    pub stage_a_plotted_pixels: u16,
    /// World ticks stage A spent, one per eight dispatch steps.
    pub stage_a_world_ticks: u16,
    /// Stage B's phase steps - `15` when the stage ran whole.
    pub stage_b_phase_steps: u8,
    /// The phases stage B drew, first and last: `15` down to `1`.
    pub stage_b_first_phase: u8,
    pub stage_b_last_phase: u8,
    /// BIOS timer ticks stage B waited in total, two per phase.
    pub stage_b_bios_ticks: u16,
    /// The shared presence counter when the transit finished. `§9.2`:
    /// "The countdown ends with the counter at zero."
    pub ended_counter: u8,
    /// Whether the sequence ran every published step in one call rather
    /// than returning early. `§9.2` makes the transition blocking and
    /// unskippable, so a completed transit always reports `true`.
    pub ran_to_completion: bool,
}

impl MoongateTransitPlayback {
    /// The playback a whole, unskipped transit produces - every published
    /// count, ending on a zeroed counter.
    pub const fn complete() -> Self {
        Self {
            stage_a_dispatch_steps: MOONGATE_TRANSIT_STAGE_A_DISPATCH_STEPS as u16,
            stage_a_plotted_pixels: MOONGATE_TRANSIT_STAGE_A_PLOTTED_PIXELS as u16,
            stage_a_world_ticks: MOONGATE_TRANSIT_STAGE_A_WORLD_TICKS as u16,
            stage_b_phase_steps: MOONGATE_TRANSIT_STAGE_B_STEPS as u8,
            stage_b_first_phase: MOONGATE_TRANSIT_STAGE_B_FIRST_PHASE,
            stage_b_last_phase: MOONGATE_TRANSIT_STAGE_B_LAST_PHASE,
            stage_b_bios_ticks: (MOONGATE_TRANSIT_STAGE_B_STEPS
                * MOONGATE_TRANSIT_STAGE_B_STEP_BIOS_TICKS as usize)
                as u16,
            ended_counter: MOONGATE_TRANSIT_END_COUNTER,
            ran_to_completion: true,
        }
    }
}

/// `overworld.md §9.2`: run the blocking transit to completion, driving
/// the **shared** gate-presence counter and handing every dispatch step to
/// `on_step`.
///
/// This is the whole sequence in one call: `§9.2` runs it "to completion
/// before the party is relocated and before any key is read", and the
/// player has no abort ([`MOONGATE_TRANSIT_ABORT_POLL_ENABLED`]), so there
/// is no resumable state and no early return.
///
/// `counter` is the same save-backed presence counter `§9.1` describes.
/// Stage B sets it to each phase from `15` down to `1`; the countdown then
/// ends with it at zero, which is why a gate mid-rise elsewhere in view is
/// driven to zero by this transit. `§9.1` states that plainly as "the
/// original's behaviour, not a defect to design around".
///
/// `on_step` is handed each step together with the counter as that step
/// leaves it, which is the phase a presentation layer draws the gate cell
/// at.
pub fn run_moongate_transit(
    counter: &mut u8,
    on_step: &mut impl FnMut(MoongateTransitStep, u8) -> io::Result<()>,
) -> io::Result<MoongateTransitPlayback> {
    let mut playback = MoongateTransitPlayback::default();
    for step in moongate_transit_steps()? {
        match step {
            MoongateTransitStep::OpeningPause { .. } => {}
            MoongateTransitStep::StageAClearCell { .. } => {
                playback.stage_a_dispatch_steps += 1;
            }
            MoongateTransitStep::StageAPlotPixel { world_tick, .. } => {
                playback.stage_a_dispatch_steps += 1;
                playback.stage_a_plotted_pixels += 1;
                playback.stage_a_world_ticks += u16::from(world_tick);
            }
            MoongateTransitStep::StageBPhase {
                phase,
                wait_bios_ticks,
            } => {
                *counter = phase;
                if playback.stage_b_phase_steps == 0 {
                    playback.stage_b_first_phase = phase;
                }
                playback.stage_b_last_phase = phase;
                playback.stage_b_phase_steps += 1;
                playback.stage_b_bios_ticks += u16::from(wait_bios_ticks);
            }
            MoongateTransitStep::ClearGateCell { .. } => {
                // "The countdown ends with the counter at zero", and only
                // then is the live cell rewritten to terrain `5`.
                *counter = MOONGATE_TRANSIT_END_COUNTER;
            }
        }
        on_step(step, *counter)?;
    }
    playback.ended_counter = *counter;

    let expected = MoongateTransitPlayback {
        ran_to_completion: false,
        ..MoongateTransitPlayback::complete()
    };
    playback.ran_to_completion = playback == expected;
    if !playback.ran_to_completion {
        return Err(io::Error::other(format!(
            "overworld.md §9.2 transit is blocking and unskippable, so it must spend every \
             published step; this run spent {playback:?} rather than {expected:?}"
        )));
    }
    Ok(playback)
}

/// One frame of the transit as a presentation layer receives it.
#[derive(Clone, Copy, Debug)]
pub struct MoongateTransitFrame<'a> {
    /// The dispatch step this frame shows.
    pub step: MoongateTransitStep,
    /// How the party is drawn on this frame.
    pub party_sprite: MoongateTransitPartySprite,
    /// The gate cell's 256 pixels as this frame leaves them: the
    /// part-dissolved cell through stage A, the `§9.1` composed frame
    /// through stage B, the ground plate once step 4 has repainted it.
    pub cell: &'a [u8],
    /// The pixels the party sprite is drawn from, when one is drawn.
    /// Through stage A that is the `0x116` slot - and it holds the shipped
    /// artwork, because every `§9.1` composition restores the slot.
    pub party_pixels: Option<&'a [u8]>,
}

/// `overworld.md §9.2`: run the transit against a tile atlas, handing each
/// frame's pixels to `on_frame`.
///
/// `atlas_pixels` is the atlas's whole pixel buffer and `ground_tile` the
/// scene's ground plate (`§9.1`: terrain `5` in ordinary play, `0x44` in
/// the endgame chamber). Stage B composes into the **real** `0x116` slot
/// through [`with_moongate_phase_scratch_tile`], which saves the slot
/// before each frame and restores it after, so the shipped artwork is
/// intact when this returns - and intact throughout stage A, which draws
/// the party from that same id.
pub fn run_moongate_transit_presentation(
    atlas_pixels: &mut [u8],
    ground_tile: usize,
    counter: &mut u8,
    on_frame: &mut impl FnMut(&MoongateTransitFrame<'_>),
) -> io::Result<MoongateTransitPlayback> {
    let tile_pixels = |tile: usize| -> io::Result<Vec<u8>> {
        let start = tile.checked_mul(TILE_ATLAS_TILE_PIXELS).ok_or_else(|| {
            io::Error::other(format!(
                "overworld.md §9.2 transit needs tile {tile}, which overflows the atlas"
            ))
        })?;
        atlas_pixels
            .get(start..start + TILE_ATLAS_TILE_PIXELS)
            .map(<[u8]>::to_vec)
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("tile atlas is missing tile {tile}"),
                )
            })
    };
    let ground = tile_pixels(ground_tile)?;
    let gate = tile_pixels(moongate_phase_gate_tile() as usize)?;
    // Touch the scratch slot once up front so a short atlas fails before
    // any frame is drawn rather than part-way through a blocking sequence.
    tile_pixels(MOONGATE_PHASE_SCRATCH_TILE)?;
    let scratch_start = MOONGATE_PHASE_SCRATCH_TILE * TILE_ATLAS_TILE_PIXELS;
    let scratch_end = scratch_start + TILE_ATLAS_TILE_PIXELS;

    let mut cell = vec![MOONGATE_TRANSIT_CLEAR_COLOUR; TILE_ATLAS_TILE_PIXELS];
    run_moongate_transit(counter, &mut |step, _phase| {
        let mut party_slot = None;
        match step {
            MoongateTransitStep::OpeningPause { .. } => {
                // The gate is still whole and the party still itself; the
                // pause only spends its world tick.
                cell.copy_from_slice(&gate);
            }
            MoongateTransitStep::StageAClearCell { colour } => {
                cell.fill(colour);
                party_slot = Some(scratch_start..scratch_end);
            }
            MoongateTransitStep::StageAPlotPixel { pixel, .. } => {
                let source = gate.get(pixel).copied().ok_or_else(|| {
                    io::Error::other(format!(
                        "overworld.md §9.2 stage A plotted pixel {pixel}, outside the cell's \
                         {TILE_ATLAS_TILE_PIXELS} pixels"
                    ))
                })?;
                cell[pixel] = source;
                party_slot = Some(scratch_start..scratch_end);
            }
            MoongateTransitStep::StageBPhase { phase, .. } => {
                // `§9.1`'s composition, into the real scratch slot and out
                // again. The helper saves the slot before the frame is
                // drawn and restores it after, so the shipped artwork of
                // `0x116` - which stage A draws the party from - survives.
                let scratch = &mut atlas_pixels[scratch_start..scratch_end];
                with_moongate_phase_scratch_tile(scratch, &ground, &gate, phase, |composed| {
                    cell.copy_from_slice(composed);
                })?;
            }
            MoongateTransitStep::ClearGateCell { .. } => {
                cell.copy_from_slice(&ground);
            }
        }
        on_frame(&MoongateTransitFrame {
            step,
            party_sprite: step.party_sprite(),
            cell: &cell,
            party_pixels: party_slot.map(|slot| &atlas_pixels[slot]),
        });
        Ok(())
    })
}

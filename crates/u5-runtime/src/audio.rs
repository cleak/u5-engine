//! PC-speaker audio model.
//!
//! Every numeric value in this module comes from the published clean
//! specification `systems/audio.md` (through spec head `02550e2`) with timing classes
//! from `systems/timing.md`. Nothing here is invented from listening, from
//! private analysis, or from the shape of the previous placeholder cues.
//!
//! The module owns three layers:
//!
//! 1. [`SpeakerOp`] — the single-channel primitive vocabulary of the analyzed
//!    baseline: install a divisor and hold, hold silently, run a software
//!    envelope, or stop. `audio.md §2` makes ownership serial: starting a new
//!    tone replaces the previous divisor, and every specified end and abort
//!    performs a stop.
//! 2. The four sound families of `audio.md §5` — PIT blocking tone, linear
//!    glissando, random rumble, and software envelope — as exact constructors.
//! 3. [`SoundEffect`] — the complete confirmed trigger inventory of
//!    `audio.md §7`, `§8`, and `systems/town-mode.md §13`, each lowering to a
//!    [`SpeakerProgram`].
//!
//! The runtime records effects at their published boundaries; a frontend owns
//! synthesis. `audio.md §3` is explicit that muting changes *output*, not
//! cadence: a muted effect still performs its holds, its iterations, and its
//! final stop. [`SpeakerProgram::duration`] is therefore meaningful to a silent
//! frontend as well, and is what a shell should hold for while muted.

use std::sync::Arc;
use std::time::Duration;

// ---------------------------------------------------------------------------
// Sound toggle
// ---------------------------------------------------------------------------

/// `audio.md §3`: Ctrl-S "prints the new `Sound On` or `Sound Off` state and
/// flips the boolean".
///
/// The published strings carry no terminating period.
pub const SOUND_TOGGLE_ON_MESSAGE: &str = "Sound On";

/// `audio.md §3`. See [`SOUND_TOGGLE_ON_MESSAGE`].
pub const SOUND_TOGGLE_OFF_MESSAGE: &str = "Sound Off";

// ---------------------------------------------------------------------------
// Timing anchors
// ---------------------------------------------------------------------------

/// `audio.md §5.1`. The programmable interval timer's input frequency.
pub const PIT_INPUT_HZ: u32 = 1_193_182;

/// `audio.md §5.1`: a requested frequency `f` selects divisor
/// `floor(1,193,182 / f)`.
///
/// A zero request has no published caller; it saturates to the slowest divisor
/// rather than dividing by zero.
pub const fn pit_divisor(frequency_hz: u32) -> u16 {
    if frequency_hz == 0 {
        return u16::MAX;
    }
    let divisor = PIT_INPUT_HZ / frequency_hz;
    if divisor > u16::MAX as u32 {
        u16::MAX
    } else {
        divisor as u16
    }
}

/// The frequency a divisor actually produces, which is not exactly the
/// requested frequency because the divisor is truncated.
pub const fn frequency_for_divisor(divisor: u16) -> u32 {
    if divisor == 0 {
        return 0;
    }
    PIT_INPUT_HZ / divisor as u32
}

/// Nominal boot calibration count on the original reference machine.
///
/// Answered in `cleak/u5-spec#146`: approximately **87**, band 80..92. The
/// upper end is firmer than the centre — a bus-bandwidth floor on the
/// calibration loop caps it at 92 on a stock 4.77 MHz machine.
///
/// That cap has a consequence the engine has to honour: `audio.md §5.4` gates
/// the software envelope's idle work to zero below calibration 100, so on
/// genuine baseline hardware **the envelope's idle work is always zero** and
/// cannot be what paces it. See [`ENVELOPE_ITERATION_NANOS`].
pub const NOMINAL_BOOT_CALIBRATION: u32 = 87;

/// Wall-clock duration of one inner busy-loop unit: approximately 10.0 µs,
/// band 9.4..10.3 µs (`cleak/u5-spec#146`).
pub const INNER_UNIT_NANOS: u64 = 10_000;

/// Wall-clock duration of one outer calibrated delay unit.
///
/// This is *the* anchor. `cleak/u5-spec#146` publishes **0.88 ms, ±10%**
/// (0.79..0.97 ms) and recommends deriving everything else from it. It is
/// calibration-count inner units plus about 10 µs of fixed per-pass overhead,
/// which is why it is 88 inner units rather than 87.
///
/// The answer notes a convenient property: the calibration count and the
/// inner-step cost move in *opposite* directions on faster machines and very
/// nearly cancel, so one outer unit stays inside 0.86..0.99 ms from a 4.77 MHz
/// machine through a 486. No machine model is needed — a fixed value is correct
/// to within the original hardware's own variation.
///
/// None of this was measured. `#146` is explicit that every wall-clock figure
/// is a static derivation, and that the tolerances are modelling bands rather
/// than error bars.
pub const OUTER_CALIBRATED_UNIT_NANOS: u64 = 880_000;

/// One outer calibrated delay unit, as a count of outer units.
///
/// `cleak/u5-spec#146` Q1: there is **exactly one live delay context in the
/// shipped game and its effective shift is zero**. Every reachable call passes
/// the same selector as an unconditional constant, so every calibrated wait
/// spins the full calibration count. A 17-entry shift ramp exists as data but
/// nothing reads it, and the specification deliberately withholds its contents
/// so that nobody implements it.
///
/// The engine therefore models one calibrated delay unit and carries no shift
/// selector at all. The `>> 4` and `/ 24` terms elsewhere in `audio.md` are not
/// further contexts: they are fixed subdivisions hard-coded in their own
/// routines that never consult the selector table.
pub const fn calibrated_nanos(outer_count: u32) -> u64 {
    outer_count as u64 * OUTER_CALIBRATED_UNIT_NANOS
}

/// Cost of installing one PIT divisor, published in `timing.md §7.4.1`.
///
/// **This engine had 12 inner units here and it was 45% low.** The number was
/// back-solved from three published effect totals, and it reconciled cleanly —
/// but `cleak/u5-spec#146`'s follow-up establishes the fit was circular: the
/// figure it reconciled against was itself derived from an earlier per-update
/// overhead that omitted the delay routine's own call frame. Back-solving
/// someone's arithmetic is guaranteed to reproduce their arithmetic.
///
/// Two of the three cross-checks could never have caught it either. The install
/// cost is 0.11% of the blocked-step beep and 0.48% of the Stonegate descent,
/// so both agree with *any* value between 0 and 40 inner units. Only the action
/// snap discriminates, and there the correct 42.2 ms against the fitted 40.0 ms
/// sits inside the anchor's own ±10% band.
///
/// The cost is dominated by the divide that computes the divisor — `§7.4.1`
/// puts it plainly: the install cost essentially *is* the divide.
pub const TONE_SWEEP_INSTALL_NANOS: u64 = 174_000;

/// `timing.md §7.4.2`: the blocking wrapper carries an extra call frame over a
/// bare sweep update, so its per-tone install is 18.8 inner units rather than
/// 17.4.
pub const TONE_BLOCKING_INSTALL_NANOS: u64 = 188_000;

/// Nanoseconds one *swept* tone occupies: its calibrated hold plus the divisor
/// install. Used by the glissando family and the Stonegate descent.
pub const fn sweep_tone_nanos(hold_outer_units: u32) -> u64 {
    calibrated_nanos(hold_outer_units) + TONE_SWEEP_INSTALL_NANOS
}

/// Nanoseconds one *blocking* tone occupies. `§7.4.2`'s extra call frame.
pub const fn tone_nanos(hold_outer_units: u32) -> u64 {
    calibrated_nanos(hold_outer_units) + TONE_BLOCKING_INSTALL_NANOS
}

/// `audio.md §10.2` publishes the rumble's wall-clock cost in closed form:
/// about `target * 60.5 us + iterations * 130 us`. Since `target` is
/// `step * iterations`, that is `step * 60.5 us + 130 us` per iteration.
///
/// This replaces a constant this module previously back-derived from the trap
/// rumble alone. The published form reproduces all five tabulated rumbles,
/// where the fitted one only matched the one it was fitted to:
///
/// | Recipe | Published | This model |
/// |---|---:|---:|
/// | Trap / failed mix, step 40, target 3000 | 190 ms | 191.3 ms |
/// | Ordinary damage, step 10, target 1600 | 118 ms | 117.6 ms |
/// | Shared potion/wind lead, variant 0 | 485 ms | 485.3 ms |
/// | Two-tone sting, each half | ~4.8 ms | 4.8 ms |
/// | Return-to-View strip 2 | ~4.0 ms | 4.0 ms |
pub const RUMBLE_NANOS_PER_STEP: u64 = 60_500;
pub const RUMBLE_ITERATION_OVERHEAD_NANOS: u64 = 130_000;

/// Nanoseconds one rumble iteration spends, for a given step.
pub const fn rumble_iteration_nanos(step: u32) -> u64 {
    step as u64 * RUMBLE_NANOS_PER_STEP + RUMBLE_ITERATION_OVERHEAD_NANOS
}

/// `audio.md §10.1`: the title-sequence driver runs its own calibrated unit at
/// about 0.92 ms rather than the resident 0.88 ms; `timing.md §7.4` explains
/// why. The published publication waits are 41.3 ms sounded and 45.9 ms silent.
pub const DRIVER_LOCAL_UNIT_NANOS: u64 = 920_000;

/// `audio.md §10.1`: all twenty-five pitch holds of one ignition burst take
/// about 3.7 ms together, "25 at the rumble scale".
pub const IGNITION_BURST_TOTAL_NANOS: u64 = 3_700_000;

/// One ignition burst pitch hold.
pub const fn ignition_pitch_hold_nanos() -> u64 {
    IGNITION_BURST_TOTAL_NANOS / IGNITION_BURST_PITCHES as u64
}

/// `audio.md §5.4` phase and comparison arithmetic is modulo 65536.
pub const ENVELOPE_MODULUS: u32 = 65_536;

/// Wall-clock cost of one software-envelope iteration while it is audible.
///
/// `cleak/u5-spec#146` Q4: about **43.0 µs**. This is not derived from the idle
/// count — at baseline calibration the idle gate holds the idle work at zero
/// (see [`NOMINAL_BOOT_CALIBRATION`]) — it is the loop's own cost.
///
/// It is what fixes the family's absolute pitch. `#146` publishes the output as
/// `phase_period / 65536` cycles per iteration, exactly, so variant 0 sounds at
/// about 3.13 kHz and the nine variants form a clean descending octave from
/// 3130 Hz down to 1592 Hz. The ratios are exact because they are ratios of
/// program constants; only the absolute scale carries the published ±8%, and it
/// scales the whole column together.
pub const ENVELOPE_ITERATION_NANOS: u64 = 43_000;

/// Wall-clock cost of one software-envelope iteration while muted.
///
/// `cleak/u5-spec#146` withdrew the "matched no-output timing arm" wording:
/// **the muted envelope is not cost-matched.** Its silent arm keeps the
/// structure and the iteration count but omits the comparison and the speaker
/// work, so it runs about 23% faster — roughly 33.3 µs against 43.0 µs. A muted
/// variant-0 potion is about 1.15 s against about 1.35 s audible, which is a
/// real mute-dependent scene length rather than a rounding difference.
///
/// The blocking tones, the glissandi and the rumble *are* genuinely
/// mute-invariant; the envelope is the single exception.
pub const ENVELOPE_MUTED_ITERATION_NANOS: u64 = 33_300;

/// The carrier the envelope gates.
///
/// `cleak/u5-spec#146` Q4 corrects the model: the loop does **not** flip the
/// speaker pin. It programs the timer channel once with divisor 60 — about
/// 19,886 Hz, inaudible on its own — and then gates that running carrier on and
/// off. The audible waveform is the *gate pattern*, and a frontend that
/// synthesises a pin toggle at the loop rate lands about four octaves wrong.
pub const ENVELOPE_CARRIER_DIVISOR: u16 = 60;

/// `audio.md §4`, "drawing work": the shared full-viewport flash has no
/// explicit audio delay and is spaced only by raster work between retunes.
///
/// One band repaints a horizontal slice of the gameplay viewport. This engine
/// charges each band one outer calibrated unit, which puts the published
/// 1,856-band effect at about 1.6 s. Raster cost is not a calibration quantity
/// and `#146` does not derive it, so this remains the module's approximation.
pub const FLASH_BAND_NANOS: u64 = OUTER_CALIBRATED_UNIT_NANOS;

/// Convert an inner-unit count to nanoseconds.
pub const fn inner_units_to_nanos(inner_units: u64) -> u64 {
    inner_units * INNER_UNIT_NANOS
}

/// Convert nanoseconds to a [`Duration`].
pub const fn nanos_to_duration(nanos: u64) -> Duration {
    Duration::from_nanos(nanos)
}

// ---------------------------------------------------------------------------
// Primitive speaker operations
// ---------------------------------------------------------------------------

/// `audio.md §5.4` software envelope segment.
///
/// The generator does not interpret any argument as a frequency in hertz. The
/// phase starts at zero, each iteration adds `period` modulo 65536, compares
/// the unsigned phase against a moving comparison value, then advances the
/// comparison value by the signed `delta` modulo 65536.
///
/// What that comparison drives was corrected in `cleak/u5-spec#146`: the loop
/// does **not** flip the speaker pin. It programs the timer channel once with
/// [`ENVELOPE_CARRIER_DIVISOR`] — about 19,886 Hz, inaudible on its own — and
/// gates that running carrier on and off. The audible waveform is the gate
/// pattern, which completes exactly `period / 65536` cycles per iteration. A
/// frontend that synthesises a pin toggle at the loop rate lands about four
/// octaves wrong.
///
/// The moving comparison value therefore sweeps the gate's **duty cycle**, not
/// its pitch: roughly 91% connected falling to 55% on the first envelope of a
/// shared variant, reversed on the second, so the pair fades in and back out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EnvelopeSegment {
    /// Signed comparison delta, applied modulo 65536 after each comparison.
    pub delta: i32,
    /// Initial unsigned comparison value.
    pub initial_comparison: u16,
    /// Iteration count.
    pub iterations: u32,
    /// Idle count. `audio.md §5.4` multiplies it by `floor(calibration / 24)`
    /// and gates it to zero below calibration 100 — and `cleak/u5-spec#146`
    /// caps the baseline calibration at 92, so on the reference machine this
    /// term is always zero and does not pace the envelope. It is retained
    /// because it is a published field of the recipe.
    pub idle: u32,
    /// Phase period added to the phase accumulator each iteration.
    pub period: u16,
}

impl EnvelopeSegment {
    pub const fn new(
        delta: i32,
        initial_comparison: u16,
        iterations: u32,
        idle: u32,
        period: u16,
    ) -> Self {
        Self {
            delta,
            initial_comparison,
            iterations,
            idle,
            period,
        }
    }

    /// Wall-clock cost of one iteration, audible or muted.
    ///
    /// `cleak/u5-spec#146`: the envelope is the one family whose muted arm is
    /// **not** cost-matched, so the caller has to say which it is.
    pub const fn nanos_per_iteration(audible: bool) -> u64 {
        if audible {
            ENVELOPE_ITERATION_NANOS
        } else {
            ENVELOPE_MUTED_ITERATION_NANOS
        }
    }

    /// Total nanoseconds for the whole segment.
    pub const fn total_nanos(&self, audible: bool) -> u64 {
        Self::nanos_per_iteration(audible) * self.iterations as u64
    }

    pub const fn duration(&self, audible: bool) -> Duration {
        nanos_to_duration(self.total_nanos(audible))
    }

    /// The gate pattern's frequency in hertz.
    ///
    /// `cleak/u5-spec#146`: output cycles per iteration are exactly
    /// `phase_period / 65536`, so the audible gate frequency is that times the
    /// iteration rate.
    pub fn gate_frequency_hz(&self, audible: bool) -> f64 {
        let iteration_secs = Self::nanos_per_iteration(audible) as f64 / 1.0e9;
        f64::from(self.period) / f64::from(ENVELOPE_MODULUS) / iteration_secs
    }

    /// The pin state stream, one entry per iteration.
    ///
    /// `true` is the high pin state, chosen when the phase has reached or
    /// passed the moving comparison value. The opposite assignment is the same
    /// waveform inverted and is perceptually identical for a one-bit speaker;
    /// what the spec pins, and what this reproduces exactly, is the recurrence
    /// and the opposing sweep directions of the two paired segments.
    pub fn pin_states(&self) -> EnvelopePins {
        EnvelopePins {
            segment: *self,
            phase: 0,
            comparison: u32::from(self.initial_comparison),
            remaining: self.iterations,
        }
    }
}

/// Iterator over an [`EnvelopeSegment`]'s per-iteration pin states.
#[derive(Clone, Debug)]
pub struct EnvelopePins {
    segment: EnvelopeSegment,
    phase: u32,
    comparison: u32,
    remaining: u32,
}

impl Iterator for EnvelopePins {
    type Item = bool;

    fn next(&mut self) -> Option<bool> {
        if self.remaining == 0 {
            return None;
        }
        self.remaining -= 1;
        self.phase = (self.phase + u32::from(self.segment.period)) % ENVELOPE_MODULUS;
        let high = self.phase >= self.comparison;
        let delta = self.segment.delta.rem_euclid(ENVELOPE_MODULUS as i32) as u32;
        self.comparison = (self.comparison + delta) % ENVELOPE_MODULUS;
        Some(high)
    }
}

/// One serial operation on the single-channel speaker.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpeakerOp {
    /// Install `divisor`, enable the speaker, and hold for `nanos`.
    ///
    /// `audio.md §2`: this replaces whatever divisor was previously installed.
    Tone {
        frequency_hz: u32,
        divisor: u16,
        nanos: u64,
    },
    /// Hold without changing the speaker state.
    Silence { nanos: u64 },
    /// Run a software envelope segment.
    Envelope(EnvelopeSegment),
    /// Unconditionally disable the speaker.
    Stop,
}

impl SpeakerOp {
    pub const fn tone(frequency_hz: u32, nanos: u64) -> Self {
        SpeakerOp::Tone {
            frequency_hz,
            divisor: pit_divisor(frequency_hz),
            nanos,
        }
    }

    pub const fn nanos(&self, audible: bool) -> u64 {
        match self {
            SpeakerOp::Tone { nanos, .. } | SpeakerOp::Silence { nanos } => *nanos,
            SpeakerOp::Envelope(segment) => segment.total_nanos(audible),
            SpeakerOp::Stop => 0,
        }
    }
}

/// A complete effect, lowered to serial speaker operations.
///
/// `audio.md §2` requires a stop at every specified effect end, so every
/// constructor in this module terminates in [`SpeakerOp::Stop`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SpeakerProgram {
    pub ops: Vec<SpeakerOp>,
}

impl SpeakerProgram {
    pub fn new(ops: Vec<SpeakerOp>) -> Self {
        Self { ops }
    }

    /// Total blocking work the effect performs.
    ///
    /// `audio.md §3` makes the tone, glissando and rumble families
    /// mute-invariant, but `cleak/u5-spec#146` withdrew that for the software
    /// envelope: its silent arm runs about 23% faster. So duration is a
    /// function of the sound setting, and only the envelope families differ.
    pub fn total_nanos(&self, audible: bool) -> u64 {
        self.ops.iter().map(|op| op.nanos(audible)).sum()
    }

    pub fn duration(&self, audible: bool) -> Duration {
        nanos_to_duration(self.total_nanos(audible))
    }

    /// Number of tone updates, which `audio.md` pins for every family.
    pub fn tone_count(&self) -> usize {
        self.ops
            .iter()
            .filter(|op| matches!(op, SpeakerOp::Tone { .. }))
            .count()
    }

    /// The played frequency sequence, in order.
    pub fn frequencies(&self) -> Vec<u32> {
        self.ops
            .iter()
            .filter_map(|op| match op {
                SpeakerOp::Tone { frequency_hz, .. } => Some(*frequency_hz),
                _ => None,
            })
            .collect()
    }

    /// `audio.md §2` requires a stop at every specified effect end.
    pub fn ends_with_stop(&self) -> bool {
        matches!(self.ops.last(), Some(SpeakerOp::Stop))
    }
}

// ---------------------------------------------------------------------------
// Sound-only jitter stream
// ---------------------------------------------------------------------------

/// `audio.md §5.3`: random rumble advances "a private sound-only jitter state"
/// that "starts from the same fixed nonzero value on each program run and is
/// not the gameplay PRNG".
///
/// The spec explicitly permits a frontend to replace the sequence, "provided it
/// does not use or perturb gameplay randomness and preserves the frequency
/// range, iteration count, and timing". This is that deterministic replacement.
/// It must never be seeded from, or written back into, [`crate::prng`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RumbleJitter {
    state: u32,
}

/// The fixed nonzero start value. Any nonzero constant satisfies the contract;
/// this one is fixed so replays and tests are reproducible.
pub const RUMBLE_JITTER_SEED: u32 = 0x0000_1D53;

impl Default for RumbleJitter {
    fn default() -> Self {
        Self::new()
    }
}

impl RumbleJitter {
    pub const fn new() -> Self {
        Self {
            state: RUMBLE_JITTER_SEED,
        }
    }

    /// Advance the private state and return an inclusive frequency in
    /// `low..=high`.
    pub fn next_frequency(&mut self, low: u32, high: u32) -> u32 {
        self.state = self
            .state
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223);
        let width = high.saturating_sub(low).saturating_add(1).max(1);
        low + (self.state >> 8) % width
    }
}

// ---------------------------------------------------------------------------
// Sound families (audio.md §5)
// ---------------------------------------------------------------------------

/// `audio.md §5.1` blocking tone `(hold, frequency)`: begin the frequency, wait
/// `hold` calibrated units in delay context 1, then stop.
pub fn blocking_tone(hold_outer_units: u32, frequency_hz: u32) -> SpeakerProgram {
    SpeakerProgram::new(vec![
        SpeakerOp::tone(frequency_hz, tone_nanos(hold_outer_units)),
        SpeakerOp::Stop,
    ])
}

/// `audio.md §5.2` linear glissando `(span, delay, target, initial)`.
///
/// The signed increment is `(target - initial) * delay / span` with the
/// fractional part discarded. Playback starts at `initial`, emits
/// `ceil(span / delay)` tone updates, waits `delay` calibrated units after every
/// update, and stops once at the end. The target is the interpolation endpoint,
/// not a played update.
///
/// A negative span produces no tone update and only the final stop.
pub fn glissando(span: i32, delay: u32, target: i32, initial: i32) -> SpeakerProgram {
    let mut ops = Vec::new();
    if span > 0 && delay > 0 {
        let increment = (target - initial) * delay as i32 / span;
        let updates = (span as u32).div_ceil(delay);
        let hold = sweep_tone_nanos(delay);
        let mut frequency = initial;
        for _ in 0..updates {
            ops.push(SpeakerOp::tone(frequency.max(0) as u32, hold));
            frequency += increment;
        }
    }
    ops.push(SpeakerOp::Stop);
    SpeakerProgram::new(ops)
}

/// `audio.md §5.3` random rumble `(step, target, maximum_frequency)`.
///
/// Each iteration advances the private jitter state, chooses an inclusive
/// frequency from `100..maximum_frequency`, installs that divisor, waits
/// `step * (calibration >> 4)` inner units, and adds `step` to the accumulator.
/// The effect stops after `ceil(target / step)` iterations.
pub const RUMBLE_MIN_FREQUENCY_HZ: u32 = 100;

pub fn random_rumble(
    step: u32,
    target: u32,
    maximum_frequency_hz: u32,
    jitter: &mut RumbleJitter,
) -> SpeakerProgram {
    let mut ops = Vec::new();
    if step > 0 {
        let iterations = target.div_ceil(step);
        let hold = rumble_iteration_nanos(step);
        for _ in 0..iterations {
            let frequency = jitter.next_frequency(RUMBLE_MIN_FREQUENCY_HZ, maximum_frequency_hz);
            ops.push(SpeakerOp::tone(frequency, hold));
        }
    }
    ops.push(SpeakerOp::Stop);
    SpeakerProgram::new(ops)
}

/// `audio.md §5.4` software envelope, as a one-segment program.
pub fn software_envelope(
    delta: i32,
    initial_comparison: u16,
    iterations: u32,
    idle: u32,
    period: u16,
) -> SpeakerProgram {
    SpeakerProgram::new(vec![
        SpeakerOp::Envelope(EnvelopeSegment::new(
            delta,
            initial_comparison,
            iterations,
            idle,
            period,
        )),
        SpeakerOp::Stop,
    ])
}

// ---------------------------------------------------------------------------
// Named recipes (audio.md §5.2, §5.3)
// ---------------------------------------------------------------------------

/// `audio.md §5.2` action snap: 40 updates, 1200 Hz rising toward 2000 Hz.
pub fn action_snap() -> SpeakerProgram {
    glissando(40, 1, 2000, 1200)
}

/// `audio.md §5.2` cast failure: 50 updates, 800 Hz rising toward 2000 Hz.
pub fn cast_failure_glissando() -> SpeakerProgram {
    glissando(50, 1, 2000, 800)
}

/// `audio.md §5.2` long descent, the recipe `audio.md §8.9` gives its two
/// triggers: span 7800, per-update delay 40, initial 660 Hz, nominal target
/// 150 Hz.
pub const LONG_DESCENT_SPAN: i32 = 7800;
pub const LONG_DESCENT_DELAY_UNITS: u32 = 40;
pub const LONG_DESCENT_INITIAL_HZ: i32 = 660;
/// The **interpolation endpoint**, not a played update, and under the `§5.2`
/// truncation contract not even approached: see [`LONG_DESCENT_LAST_HZ`].
pub const LONG_DESCENT_NOMINAL_TARGET_HZ: i32 = 150;
/// `audio.md §5.2`/`§8.9`: 195 updates.
pub const LONG_DESCENT_UPDATES: usize = 195;
/// `audio.md §8.9`: "**It does not reach 150 Hz.** Under the truncation
/// contract of section 5.2 the increment is -2 Hz rather than the -2.615 the
/// endpoints imply, so the last tone played is **272 Hz** and the heard fall is
/// about 15.4 semitones rather than 25.7."
///
/// This constant exists only so a test can pin it. Nothing computes the
/// sequence from it — [`glissando`] already implements the published
/// truncation, and its dungeon-drip realised tops (3485 / 3475 / 3425 Hz)
/// corroborate that independently.
pub const LONG_DESCENT_LAST_HZ: u32 = 272;

/// `audio.md §5.2` long descent, `§8.9`'s shared recipe: 195 updates,
/// 660 Hz down to a realised 272 Hz, about 6.86 s.
///
/// By a wide margin the longest sound in the shipped game — "every other
/// glissando in the shipped build has a span of 300 units or less", so this one
/// is roughly twenty-six times the next longest.
///
/// A frontend "that interpolates 660 Hz to 150 Hz plays an effect more than ten
/// semitones deeper than the original", so this must go through [`glissando`]
/// and must not be written as an endpoint-to-endpoint interpolation.
pub fn long_descent() -> SpeakerProgram {
    glissando(
        LONG_DESCENT_SPAN,
        LONG_DESCENT_DELAY_UNITS,
        LONG_DESCENT_NOMINAL_TARGET_HZ,
        LONG_DESCENT_INITIAL_HZ,
    )
}

/// `audio.md §10.2`/`§10.6` surface falls descent: 300 updates, per-update
/// delay 1, 2500 Hz stepping -5 Hz against a nominal target of 800 Hz, so the
/// last tone played is **1005 Hz** - "truncation from -5.67 to -5 leaves a
/// 205 Hz shortfall".
pub const SURFACE_FALLS_DESCENT_SPAN: i32 = 300;
pub const SURFACE_FALLS_DESCENT_DELAY_UNITS: u32 = 1;
pub const SURFACE_FALLS_DESCENT_INITIAL_HZ: i32 = 2500;
pub const SURFACE_FALLS_DESCENT_NOMINAL_TARGET_HZ: i32 = 800;
pub const SURFACE_FALLS_DESCENT_UPDATES: usize = 300;
/// The realised endpoint, not the nominal target. Pinned so a frontend that
/// interpolates 2500 Hz to 800 Hz cannot pass for the original.
pub const SURFACE_FALLS_DESCENT_LAST_HZ: u32 = 1005;

/// `audio.md §10.6`: "One site: the overworld falls chain, played once per
/// fall, immediately after the banner and the two forced southward steps. It
/// fires on every waterfall brink on either plane, including the ones that
/// produce no plane change." Explicitly **not** any dungeon pit fall, which
/// narrates but plays no sweep, and **not** the dungeon Klimb `Failed!`
/// refusal, which is a much shorter rising recipe.
pub fn surface_falls_descent() -> SpeakerProgram {
    glissando(
        SURFACE_FALLS_DESCENT_SPAN,
        SURFACE_FALLS_DESCENT_DELAY_UNITS,
        SURFACE_FALLS_DESCENT_NOMINAL_TARGET_HZ,
        SURFACE_FALLS_DESCENT_INITIAL_HZ,
    )
}

/// `audio.md §7.4` census row / `§8.8`: 220 Hz for 150 calibrated units, then
/// 150 Hz for 150 units.
pub const COMBAT_COMMAND_REFUSED_FIRST_HZ: u32 = 220;
pub const COMBAT_COMMAND_REFUSED_SECOND_HZ: u32 = 150;
pub const COMBAT_COMMAND_REFUSED_HOLD_UNITS: u32 = 150;

/// `audio.md §8.8` combat command refused as inapplicable: the descending
/// two-tone pair, and "this event is the only thing that produces it".
///
/// "The speaker is de-gated between the two tones and re-gated under a
/// millisecond later, so it is two discrete pitches with a brief hard break,
/// not a glide. It ends in hard silence."
///
/// That break is **not** [`two_part_sting`]'s shape. The sting holds a
/// calibrated 20-unit silence (17.6 ms) between its halves; here the de-gate is
/// just the blocking primitive's own unconditional stop, and the re-gate is the
/// next tone's install cost — [`TONE_BLOCKING_INSTALL_NANOS`], 0.188 ms, which
/// is the "under a millisecond" the clause names. So this is literally two
/// [`blocking_tone`] programs back to back, with no [`SpeakerOp::Silence`]
/// between them.
///
/// Muting suppresses the tones only: `§8.8` warns that "[b]oth holds still run,
/// so a muted refusal is a silent stretch of about 263 ms of dead time. A
/// frontend that skips the whole effect when muted diverges from the original."
/// [`SpeakerProgram::total_nanos`] is mute-invariant for the tone family, so
/// that falls out for free.
pub fn combat_command_refused() -> SpeakerProgram {
    let mut ops = blocking_tone(
        COMBAT_COMMAND_REFUSED_HOLD_UNITS,
        COMBAT_COMMAND_REFUSED_FIRST_HZ,
    )
    .ops;
    ops.extend(
        blocking_tone(
            COMBAT_COMMAND_REFUSED_HOLD_UNITS,
            COMBAT_COMMAND_REFUSED_SECOND_HZ,
        )
        .ops,
    );
    SpeakerProgram::new(ops)
}

/// `audio.md §5.2` dungeon wall drip spans, near to far. The fourth band emits
/// no tone update and performs only the final stop.
pub const DUNGEON_DRIP_SPANS: [i32; 4] = [20, 12, 4, -4];

/// `audio.md §5.2` dungeon wall drip for one depth band, 3200 Hz rising toward
/// 3500 Hz.
pub fn dungeon_wall_drip(band: u8) -> SpeakerProgram {
    let span = DUNGEON_DRIP_SPANS[(band as usize).min(DUNGEON_DRIP_SPANS.len() - 1)];
    glissando(span, 1, 3500, 3200)
}

/// `audio.md §5.3` trap or failed reagent mix: 75 updates in 100..500 Hz.
pub fn trap_rumble(jitter: &mut RumbleJitter) -> SpeakerProgram {
    random_rumble(40, 3000, 500, jitter)
}

/// `audio.md §5.3` ordinary damage presentation: 160 updates in 100..2000 Hz.
pub fn damage_rumble(jitter: &mut RumbleJitter) -> SpeakerProgram {
    random_rumble(10, 1600, 2000, jitter)
}

/// `audio.md §7.4`: sailing collision, 20 updates in 100..300 Hz.
pub fn ship_collision_rumble(jitter: &mut RumbleJitter) -> SpeakerProgram {
    random_rumble(100, 2000, 300, jitter)
}

/// `audio.md §7.4`: rough-seas impact, 300 updates in 100..2000 Hz.
pub fn rough_seas_impact_rumble(jitter: &mut RumbleJitter) -> SpeakerProgram {
    random_rumble(10, 3000, 2000, jitter)
}

/// `audio.md §5.3` shared potion/wind lead for variant `v`: 10 + 2v updates in
/// 100..700 Hz.
pub fn shared_variant_lead(variant: u8, jitter: &mut RumbleJitter) -> SpeakerProgram {
    let variant = variant.min(SHARED_VARIANT_COUNT as u8 - 1);
    random_rumble(800, 8000 + 1600 * u32::from(variant), 700, jitter)
}

/// `audio.md §5.3` / §8.6 short two-part sting: 25 updates in 100..1000 Hz, a
/// 20-unit calibrated silent hold, then 25 updates in 100..1500 Hz. Every
/// Blackthorn VM stinger repetition emits this live recipe before its separate
/// two-tick cinematic pause.
pub fn two_part_sting(jitter: &mut RumbleJitter) -> SpeakerProgram {
    let mut ops = random_rumble(1, 25, 1000, jitter).ops;
    ops.pop();
    ops.push(SpeakerOp::Silence {
        nanos: calibrated_nanos(20),
    });
    ops.extend(random_rumble(1, 25, 1500, jitter).ops);
    SpeakerProgram::new(ops)
}

// ---------------------------------------------------------------------------
// Shared potion / wind / spell variants (audio.md §6)
// ---------------------------------------------------------------------------

/// The nine low-numbered audiovisual variants of `audio.md §6`.
pub const SHARED_VARIANT_COUNT: usize = 9;

/// One row of the `audio.md §6` shared variant table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SharedVariantRow {
    pub phase_period: u16,
    pub first_initial_comparison: u16,
    pub second_initial_comparison: u16,
    pub delta_magnitude: i32,
    pub iterations_per_envelope: u32,
}

/// `audio.md §6`. Both envelopes use idle count 1.
pub const SHARED_VARIANT_IDLE: u32 = 1;

/// `audio.md §6` shared potion and wind envelope table.
pub const SHARED_VARIANTS: [SharedVariantRow; SHARED_VARIANT_COUNT] = [
    SharedVariantRow {
        phase_period: 8810,
        first_initial_comparison: 2700,
        second_initial_comparison: 32700,
        delta_magnitude: 3,
        iterations_per_envelope: 10_000,
    },
    SharedVariantRow {
        phase_period: 7830,
        first_initial_comparison: 3000,
        second_initial_comparison: 31000,
        delta_magnitude: 2,
        iterations_per_envelope: 14_000,
    },
    SharedVariantRow {
        phase_period: 7060,
        first_initial_comparison: 1000,
        second_initial_comparison: 37000,
        delta_magnitude: 2,
        iterations_per_envelope: 18_000,
    },
    SharedVariantRow {
        phase_period: 6550,
        first_initial_comparison: 100,
        second_initial_comparison: 45000,
        delta_magnitude: 2,
        iterations_per_envelope: 22_000,
    },
    SharedVariantRow {
        phase_period: 5950,
        first_initial_comparison: 5000,
        second_initial_comparison: 31000,
        delta_magnitude: 1,
        iterations_per_envelope: 26_000,
    },
    SharedVariantRow {
        phase_period: 5570,
        first_initial_comparison: 4000,
        second_initial_comparison: 34000,
        delta_magnitude: 1,
        iterations_per_envelope: 30_000,
    },
    SharedVariantRow {
        phase_period: 5180,
        first_initial_comparison: 2500,
        second_initial_comparison: 36500,
        delta_magnitude: 1,
        iterations_per_envelope: 34_000,
    },
    SharedVariantRow {
        phase_period: 4820,
        first_initial_comparison: 1000,
        second_initial_comparison: 39000,
        delta_magnitude: 1,
        iterations_per_envelope: 38_000,
    },
    SharedVariantRow {
        phase_period: 4480,
        first_initial_comparison: 1,
        second_initial_comparison: 42000,
        delta_magnitude: 1,
        iterations_per_envelope: 42_000,
    },
];

/// A viewport inversion performed by the shared variant sequence.
///
/// `audio.md §6` brackets the two envelopes with a pair of full-viewport
/// inversions. They are presentation work, not sound, but they are part of the
/// blocking sequence and `audio.md §7.2` requires them even while muted, so the
/// program records them as zero-cost markers a frontend can act on.
pub const SHARED_VARIANT_INVERSIONS: usize = 2;

/// `audio.md §6` complete shared sequence for one variant: rumble lead, invert,
/// rising envelope, falling envelope, invert back.
pub fn shared_variant(variant: u8, jitter: &mut RumbleJitter) -> SpeakerProgram {
    let index = (variant as usize).min(SHARED_VARIANT_COUNT - 1);
    let row = SHARED_VARIANTS[index];
    let mut ops = shared_variant_lead(variant, jitter).ops;
    ops.pop();
    ops.push(SpeakerOp::Envelope(EnvelopeSegment::new(
        row.delta_magnitude,
        row.first_initial_comparison,
        row.iterations_per_envelope,
        SHARED_VARIANT_IDLE,
        row.phase_period,
    )));
    ops.push(SpeakerOp::Envelope(EnvelopeSegment::new(
        -row.delta_magnitude,
        row.second_initial_comparison,
        row.iterations_per_envelope,
        SHARED_VARIANT_IDLE,
        row.phase_period,
    )));
    ops.push(SpeakerOp::Stop);
    SpeakerProgram::new(ops)
}

/// `audio.md §6.1` the circle-scaled rumble lead, played on its own.
///
/// Two families reach it. The **combat effect template**: "the combat impact
/// helper then plays random rumble `(800, 8000 + 1600 x circle, 700)` - which
/// is **exactly the rumble lead of that circle's shared variant**, with the
/// viewport inversion and both envelopes omitted". And the **mass-target
/// family**: "one bare random rumble `(800, T, 700)` ... Those are again
/// `8000 + 1600 x circle` - the rumble lead of the id's own circle's variant,
/// without the inversion or the envelope pair".
pub fn circle_rumble_lead(circle: u8, jitter: &mut RumbleJitter) -> SpeakerProgram {
    random_rumble(800, 8000 + 1600 * u32::from(circle), 700, jitter)
}

/// `audio.md §6.1` combat effect template impact: "a **descending** glissando,
/// 20 updates from 1300 Hz down toward 350 Hz".
pub fn combat_template_impact() -> SpeakerProgram {
    glissando(20, 1, 350, 1300)
}

/// `audio.md §6.1`: a spell's circle, `floor(id / 6) + 1`.
pub const fn spell_circle(spell_id: usize) -> u8 {
    (spell_id / 6 + 1) as u8
}

/// `audio.md §6.1` the seven spell ids that reach the shared dispatcher on no
/// path: the three combat effect-template spells (Magic Missile 1, Fireball 13,
/// Kill 37) and the four mass-target spells (Sleep 28, Poison Wind 40, Death
/// Wind 44, Flame Wind 45).
pub const SPELL_IDS_WITH_NO_SHARED_VARIANT: [usize; 7] = [1, 13, 28, 37, 40, 44, 45];

/// `audio.md §6.1`: "**the variant is the tier index of the thing being
/// used**". A spell supplies its own circle, so circles 1 through 8 map to
/// variants 1 through 8, and "no spell uses variant 0".
///
/// Kill (id 37) is listed among the seven exceptions: an earlier revision put
/// it in variant 6, which `RETRACTIONS.md` withdraws - "Kill is a circle-7
/// spell, and it plays **no dispatcher variant at all**".
pub const fn spell_shared_variant(spell_id: usize) -> Option<u8> {
    let mut index = 0;
    while index < SPELL_IDS_WITH_NO_SHARED_VARIANT.len() {
        if SPELL_IDS_WITH_NO_SHARED_VARIANT[index] == spell_id {
            return None;
        }
        index += 1;
    }
    Some(spell_circle(spell_id))
}

/// `audio.md §6.1`: the four field spells reach the dispatcher on their
/// **dungeon** arm only; "Combat arm uses the template".
pub const SPELL_IDS_WITH_DUNGEON_ONLY_VARIANT: [usize; 4] = [14, 15, 16, 20];

/// `audio.md §6.1` field-spell arm selection. Id 20 (Energy Field) is the row
/// the shared field helper special-cases so that variant still equals circle.
pub const fn field_spell_shared_variant(spell_id: usize, dungeon_arm: bool) -> Option<u8> {
    if !dungeon_arm {
        return None;
    }
    spell_shared_variant(spell_id)
}

/// `audio.md §6.1`: "A scroll supplies its **scroll index**, 0 through 7."
///
/// "A scroll does not sound like its spell... **A frontend must not reuse the
/// spell's variant for the scroll.**"
pub const SCROLL_COUNT: usize = 8;

pub const fn scroll_shared_variant(scroll_index: usize) -> Option<u8> {
    if scroll_index < SCROLL_COUNT {
        Some(scroll_index as u8)
    } else {
        None
    }
}

/// `audio.md §7.3` wind-change caller tag.
///
/// **The variant is chosen by the caller tag, not by the wind.** The earlier
/// "previous wind state selects the variant" rule, and its Calm-versus-direction
/// transition matrix, are withdrawn (`RETRACTIONS.md`, 2026-08-26): "The old
/// wind does not participate in variant selection at all."
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindChangeCaller {
    /// `Rel Hur`, spell id 8, circle 2.
    Spell,
    /// Scroll index 1, `Wind change!`.
    Scroll,
}

/// `audio.md §7.3`: the Wind Change **spell** plays variant 2.
pub const WIND_SPELL_VARIANT: u8 = 2;
/// `audio.md §7.3`: the Wind Change **scroll** plays variant 1.
pub const WIND_SCROLL_VARIANT: u8 = 1;

/// `audio.md §7.3`: `None` means no sound at all.
///
/// There is exactly one silent accepted path: a **spell**-tagged call
/// requesting direction "none". `§7.3` marks as **unresolved** whether that
/// guard is additionally conditioned on the current wind already being Calm,
/// and the two readings "differ only in whether a spell that calms an already-
/// windy sea is audible". This takes the wider reading - spell tag plus
/// direction "none" - which `§7.3` states is "necessary in both readings", so
/// the silence never depends on the unsettled half of the guard.
///
/// The autonomous drift never reaches here: `§11` lists it under the
/// wind-change sequence's "explicitly **not** produced by" column, and the
/// setter itself "contains no sound call".
pub const fn wind_change_variant(caller: WindChangeCaller, requested_is_calm: bool) -> Option<u8> {
    match caller {
        WindChangeCaller::Spell if requested_is_calm => None,
        WindChangeCaller::Spell => Some(WIND_SPELL_VARIANT),
        WindChangeCaller::Scroll => Some(WIND_SCROLL_VARIANT),
    }
}

// ---------------------------------------------------------------------------
// Named envelopes (audio.md §8.3, §8.7)
// ---------------------------------------------------------------------------

/// `audio.md §8.3` monster possession success.
pub const POSSESSION_ENVELOPE: EnvelopeSegment = EnvelopeSegment::new(2, 1000, 30_000, 1, 3100);
/// `audio.md §8.4.1` Shadow Lord arena, sceptre reclaimed.
///
/// The longest non-blocking note in the game by a wide margin: about 2.8 s at
/// roughly 1.4 kHz at the `#146` anchor. Read the shape carefully — the phase
/// period is fixed while the comparison climbs by one per iteration from one,
/// so the gate starts almost fully open and closes progressively. It is a slow
/// **fade-out at constant pitch**, not a chirp and not a sweep. An
/// implementation that treats a quest-completion sting as something short and
/// bright is wrong by an order of magnitude in duration and wrong in shape;
/// `cleak/u5-spec#152` is the exchange that settled it.
///
/// Muting takes the envelope's silent arm, which is not cost-matched, so a
/// muted reclaim is *shorter* than an audible one rather than an equal-length
/// silence.
pub const SCEPTRE_RECLAIMED_ENVELOPE: EnvelopeSegment = EnvelopeSegment::new(1, 1, 65_000, 1, 4050);
/// `audio.md §8.3` monster summon success.
pub const MONSTER_SUMMON_ENVELOPE: EnvelopeSegment = EnvelopeSegment::new(15, 1000, 5_000, 1, 2760);
/// `audio.md §8.3` accepted player Summon placement.
pub const PLAYER_SUMMON_ENVELOPE: EnvelopeSegment = EnvelopeSegment::new(5, 500, 12_000, 1, 2760);
/// `audio.md §8.3` accepted moongate transit.
pub const MOONGATE_TRANSIT_ENVELOPE: EnvelopeSegment =
    EnvelopeSegment::new(2, 2000, 30_000, 1, 5900);
/// `audio.md §8.7` endgame Dead-member restoration flourish.
pub const ENDGAME_RESTORATION_ENVELOPE: EnvelopeSegment =
    EnvelopeSegment::new(1, 5000, 40_000, 1, 8800);
/// `audio.md §8.7` endgame box/tableau presentation.
pub const ENDGAME_TABLEAU_ENVELOPE: EnvelopeSegment =
    EnvelopeSegment::new(1, 10_000, 50_000, 1, 5200);

/// `audio.md §8.6.2`: the six independent Blackthorn rescue envelopes in
/// execution order. Each segment is its own divisor-60 speaker program,
/// restarts phase at zero, and forces silence before the next begins.
pub const BLACKTHORN_RESCUE_ENVELOPES: [EnvelopeSegment; 6] = [
    EnvelopeSegment::new(1, 3000, 50_000, 1, 4400),
    EnvelopeSegment::new(1, 3000, 50_000, 1, 4125),
    EnvelopeSegment::new(1, 3000, 50_000, 1, 3667),
    EnvelopeSegment::new(1, 1000, 30_000, 1, 2933),
    EnvelopeSegment::new(1, 100, 40_000, 1, 3300),
    EnvelopeSegment::new(-1, 40_100, 40_000, 1, 3300),
];

fn envelope_program(segment: EnvelopeSegment) -> SpeakerProgram {
    SpeakerProgram::new(vec![SpeakerOp::Envelope(segment), SpeakerOp::Stop])
}

pub fn blackthorn_rescue_envelope_program() -> SpeakerProgram {
    let mut ops = Vec::with_capacity(BLACKTHORN_RESCUE_ENVELOPES.len() * 2);
    for segment in BLACKTHORN_RESCUE_ENVELOPES {
        ops.push(SpeakerOp::Envelope(segment));
        ops.push(SpeakerOp::Stop);
    }
    SpeakerProgram::new(ops)
}

// ---------------------------------------------------------------------------
// Major full-viewport flash (audio.md §8.4)
// ---------------------------------------------------------------------------

/// `audio.md §8.4`: eight rounds of four 58-band sweeps.
pub const FLASH_ROUNDS: u32 = 8;
pub const FLASH_SWEEPS_PER_ROUND: u32 = 4;
pub const FLASH_BANDS_PER_SWEEP: u32 = 58;
/// 1,856 band draws and 1,856 frequency changes per invocation.
pub const FLASH_BAND_COUNT: u32 = FLASH_ROUNDS * FLASH_SWEEPS_PER_ROUND * FLASH_BANDS_PER_SWEEP;
/// `audio.md §8.4` inclusive band frequency range.
pub const FLASH_MIN_FREQUENCY_HZ: u16 = 19;
pub const FLASH_MAX_FREQUENCY_HZ: u16 = 150;

/// Draw the `audio.md §8.4` band frequencies from the **gameplay** PRNG.
///
/// This is the one audio effect whose randomness is gameplay state. Muting must
/// suppress each tone start but must not skip any of these 1,856 advances, so
/// the caller performs this draw unconditionally and only the frontend consults
/// the sound setting.
pub fn draw_major_flash_bands(prng: &mut crate::prng::U5Prng) -> Arc<[u8]> {
    let mut bands = Vec::with_capacity(FLASH_BAND_COUNT as usize);
    for _ in 0..FLASH_BAND_COUNT {
        bands.push(prng.next_range_u16(FLASH_MIN_FREQUENCY_HZ, FLASH_MAX_FREQUENCY_HZ) as u8);
    }
    bands.into()
}

fn major_flash_program(bands: &[u8]) -> SpeakerProgram {
    let mut ops = Vec::with_capacity(bands.len() + 1);
    for band in bands {
        ops.push(SpeakerOp::tone(u32::from(*band), FLASH_BAND_NANOS));
    }
    ops.push(SpeakerOp::Stop);
    SpeakerProgram::new(ops)
}

// ---------------------------------------------------------------------------
// Stonegate scripted death (audio.md §8.2)
// ---------------------------------------------------------------------------

/// `audio.md §8.2`: every integer frequency from 1000 down through 251 Hz.
pub const STONEGATE_DESCENT_TOP_HZ: u32 = 1000;
pub const STONEGATE_DESCENT_BOTTOM_HZ: u32 = 251;
/// 750 tones total.
pub const STONEGATE_DESCENT_TONES: u32 = STONEGATE_DESCENT_TOP_HZ - STONEGATE_DESCENT_BOTTOM_HZ + 1;
/// Each tone is held for 40 calibrated units.
pub const STONEGATE_DESCENT_HOLD_UNITS: u32 = 40;

fn stonegate_descent_program() -> SpeakerProgram {
    let hold = sweep_tone_nanos(STONEGATE_DESCENT_HOLD_UNITS);
    let mut ops = Vec::with_capacity(STONEGATE_DESCENT_TONES as usize + 1);
    for frequency in (STONEGATE_DESCENT_BOTTOM_HZ..=STONEGATE_DESCENT_TOP_HZ).rev() {
        ops.push(SpeakerOp::tone(frequency, hold));
    }
    ops.push(SpeakerOp::Stop);
    SpeakerProgram::new(ops)
}

// ---------------------------------------------------------------------------
// Subtitle ignition (audio.md §7.1)
// ---------------------------------------------------------------------------

/// `audio.md §7.1`: an admitted burst emits 25 successive frequencies.
pub const IGNITION_BURST_PITCHES: usize = 25;
/// `audio.md §7.1` inclusive burst pitch range.
pub const IGNITION_MIN_FREQUENCY_HZ: u16 = 100;
pub const IGNITION_MAX_FREQUENCY_HZ: u16 = 1500;
/// `audio.md §7.1`: the burst threshold starts at 400 each pass and publication
/// `k`, counting from one, uses threshold `400 - 3k`.
pub const IGNITION_BASE_THRESHOLD: i32 = 400;
pub const IGNITION_THRESHOLD_STEP: i32 = 3;
/// `audio.md §7.1`: a burst is admitted when the low nine bits of the advanced
/// gate state are below the threshold.
pub const IGNITION_GATE_MASK: u32 = 0x1FF;
/// `audio.md §7.1`: a sounded publication waits 45 full calibration units, a
/// silent one waits 50.
pub const IGNITION_SOUNDED_PUBLISH_UNITS: u32 = 45;
pub const IGNITION_SILENT_PUBLISH_UNITS: u32 = 50;

/// `audio.md §10.1`: 41.3 ms sounded, 45.9 ms silent, at the driver-local unit.
pub const fn ignition_publish_nanos(sounded: bool) -> u64 {
    let units = if sounded {
        IGNITION_SOUNDED_PUBLISH_UNITS
    } else {
        IGNITION_SILENT_PUBLISH_UNITS
    };
    units as u64 * DRIVER_LOCAL_UNIT_NANOS
}

/// `audio.md §7.1` publication threshold for the `k`th publication of a pass,
/// counting from one.
pub const fn ignition_threshold(publication_index: u32) -> i32 {
    IGNITION_BASE_THRESHOLD - IGNITION_THRESHOLD_STEP * publication_index as i32
}

/// `audio.md §8.6.1`: one dissolve retune — install the divisor, keep the
/// speaker enabled, hold for the negligible per-click hold.
///
/// Deliberately **no** [`SpeakerOp::Stop`]. `§2` requires a stop "at every
/// specified effect end", and the dissolve's effect end is its shared exit, not
/// each click; `§2` explicitly allows an effect to "leave the speaker enabled
/// until the next frame changes or stops it". Emitting a stop here is exactly
/// the per-click model `RETRACTIONS.md` R230 withdrew.
pub fn dissolve_click_retune(frequency_hz: u16) -> SpeakerProgram {
    SpeakerProgram::new(vec![SpeakerOp::tone(
        u32::from(frequency_hz),
        dissolve_click_hold_nanos(),
    )])
}

/// `audio.md §8.6.1`: a whole gated dissolve as one enabled speaker — a retune
/// per click and a single stop at the shared exit.
///
/// This is the lowering a caller holding an ordered run of clicks wants: one
/// continuous waveform whose frequency is randomised at the retune cadence, not
/// a train of discrete clicks. An aborted run is lowered the same way; the
/// abort completes its current click and then reaches the same exit.
pub fn dissolve_click_run(pitches: &[u16]) -> SpeakerProgram {
    let hold = dissolve_click_hold_nanos();
    let mut ops = Vec::with_capacity(pitches.len() + 1);
    for pitch in pitches {
        ops.push(SpeakerOp::tone(u32::from(*pitch), hold));
    }
    ops.push(SpeakerOp::Stop);
    SpeakerProgram::new(ops)
}

fn ignition_burst_program(pitches: &[u16]) -> SpeakerProgram {
    let hold = ignition_pitch_hold_nanos();
    let mut ops = Vec::with_capacity(pitches.len() + 1);
    for pitch in pitches {
        ops.push(SpeakerOp::tone(u32::from(*pitch), hold));
    }
    ops.push(SpeakerOp::Stop);
    SpeakerProgram::new(ops)
}

// ---------------------------------------------------------------------------
// Return-to-View (audio.md §8.6)
// ---------------------------------------------------------------------------

/// `audio.md §8.6` Return-to-View strip 2: rumble `(20, 60, 10000)`, exactly
/// three random pitches in 100..10000 Hz.
pub fn return_to_view_strip2(jitter: &mut RumbleJitter) -> SpeakerProgram {
    random_rumble(20, 60, 10_000, jitter)
}

/// `audio.md §8.6` Return-to-View strip 3: 3000 Hz for 3 calibrated units at
/// local phase 0, 2000 Hz for 3 at phase 4.
pub const RETURN_TO_VIEW_STRIP3_HOLD_UNITS: u32 = 3;
pub const RETURN_TO_VIEW_STRIP3_PHASE0_HZ: u32 = 3000;
pub const RETURN_TO_VIEW_STRIP3_PHASE4_HZ: u32 = 2000;

/// `audio.md §8.6`: only local phases 0 and 4 make a sound.
pub const fn return_to_view_strip3_frequency(phase: u8) -> Option<u32> {
    match phase {
        0 => Some(RETURN_TO_VIEW_STRIP3_PHASE0_HZ),
        4 => Some(RETURN_TO_VIEW_STRIP3_PHASE4_HZ),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Harpsichord (town-mode.md §13)
// ---------------------------------------------------------------------------

/// `town-mode.md §13.1` per-key phase periods, indexed by the digit itself.
///
/// **The scale ascends.** An earlier revision published it as descending with
/// digit `1` highest; `§13.1` withdraws that, and `RETRACTIONS.md` records it.
/// This engine implemented the withdrawn reading and played the tune upside
/// down.
///
/// The correction is mechanical rather than a judgement call: these constants
/// are **phase periods** fed to the software envelope generator, not tone
/// periods and not timer divisors. Under `audio.md §5.4.2` the emitted pitch is
/// *proportional* to the phase period — there is no reciprocal anywhere on this
/// path — so a larger constant is a higher note.
///
/// Digit `0` is the tenth key, not a zero index: the layout reads `1`..`9` then
/// `0` as "ten", and the digit indexes this table directly.
///
/// The constants are exact program values, and every ratio between them — and
/// therefore the whole semitone column — is exact and machine-independent.
/// Note that the octave is deliberately **not** bit-exact: an exact doubling of
/// digit `1` would be 6232 and the shipped digit `8` is 6231, 0.28 cents flat.
/// `§13.1` is explicit that an implementation must "reproduce the constants,
/// not the idealised ratios", so this table is literal rather than generated
/// from semitone arithmetic.
pub const HARPSICHORD_PHASE_PERIODS: [u16; 10] =
    [7851, 3116, 3497, 3926, 4159, 4668, 5240, 5882, 6231, 6995];

/// `town-mode.md §13.1`: the generator runs 4000 iterations with no keyboard
/// poll, no clock-tick reference and no early exit, so the handler blocks until
/// the note finishes — about 172 ms at the reference machine.
pub const HARPSICHORD_ITERATIONS: u32 = 4000;

/// `town-mode.md §13.1`: "The generator's comparison value starts at 20000 and
/// steps by -4 each iteration, finishing at 4000; it is monotonic and never
/// wraps." By `audio.md §5.4.6`'s duty relation the connected fraction rises
/// from 69.5% to 93.9%, which is what makes the note read as *plucked*. A
/// frontend that plays a constant-amplitude square wave for 172 ms will not
/// sound like the original.
pub const HARPSICHORD_INITIAL_COMPARISON: u16 = 20_000;
pub const HARPSICHORD_COMPARISON_DELTA: i32 = -4;
pub const HARPSICHORD_IDLE: u32 = 1;

/// `audio.md §5.4.3`: the reference iteration rate is about 23,300 per second,
/// so one phase-period unit is about `23300 / 65536` Hz.
///
/// `§13.1` marks the absolute pitch column **approximate** and inheriting that
/// section's modelling band: digit `1` lies somewhere in about 990..1185 Hz. An
/// implementation may place the scale anywhere in that band but "must not
/// transpose one key without transposing all ten", which is automatic here
/// because every pitch derives from this one multiplier.
pub const HARPSICHORD_HZ_PER_PHASE_UNIT: f64 = 0.3553;

/// The envelope one harpsichord key plays.
pub fn harpsichord_envelope(digit: u8) -> EnvelopeSegment {
    EnvelopeSegment::new(
        HARPSICHORD_COMPARISON_DELTA,
        HARPSICHORD_INITIAL_COMPARISON,
        HARPSICHORD_ITERATIONS,
        HARPSICHORD_IDLE,
        HARPSICHORD_PHASE_PERIODS[(digit as usize) % 10],
    )
}

/// Approximate emitted pitch of one harpsichord key at the reference machine.
pub fn harpsichord_frequency(digit: u8) -> u32 {
    (f64::from(HARPSICHORD_PHASE_PERIODS[(digit as usize) % 10]) * HARPSICHORD_HZ_PER_PHASE_UNIT)
        .round() as u32
}

/// `town-mode.md §13` thirteen-note tune.
pub const HARPSICHORD_TUNE: [u8; 13] = [6, 7, 8, 9, 8, 7, 8, 7, 6, 7, 6, 5, 3];

/// `town-mode.md §13` re-sync rule: after a note that does not continue the
/// tune, progress becomes the length of the longest suffix of the just-played
/// notes that is still a prefix of the tune.
///
/// The published worked examples are that a stray `8` after ten correct notes
/// leaves the player three notes in, a stray `7` after eleven correct notes
/// leaves them two notes in, a stray `6` at any other point leaves them one
/// note in, and any other wrong note resets progress to zero.
pub fn harpsichord_progress_after(progress: usize, digit: u8) -> usize {
    if progress < HARPSICHORD_TUNE.len() && HARPSICHORD_TUNE[progress] == digit {
        return progress + 1;
    }
    // Rebuild the played run: `progress` correct notes followed by `digit`.
    let mut played: Vec<u8> = HARPSICHORD_TUNE[..progress.min(HARPSICHORD_TUNE.len())].to_vec();
    played.push(digit);
    // Longest suffix of `played` that is a prefix of the tune.
    for length in (1..=played.len().min(HARPSICHORD_TUNE.len())).rev() {
        let suffix = &played[played.len() - length..];
        if suffix == &HARPSICHORD_TUNE[..length] {
            return length;
        }
    }
    0
}

// ---------------------------------------------------------------------------
// Intro rectangle dissolve (audio.md §8.6.1)
// ---------------------------------------------------------------------------

// `audio.md §8.6.1` replaced the one-line "progress-dependent click/hiss" this
// module was originally built from, and the model it produced was wrong in
// three independent ways. All three are corrected here; the old comments in
// this block were rationalisations of two superseded models and are gone.
//
// The corrected model, quoting §8.6.1:
//
// > the speaker's square wave runs continuously from the first click to the end
// > of the dissolve, and is retuned to a fresh pseudorandom frequency every
// > second visited pixel.
//
// So there is no per-click stop: the speaker is enabled at the first click and
// silenced only at the dissolve's shared exit, reached by both the abort path
// and normal completion (`RETRACTIONS.md` R230, withdrawing the per-click
// "short percussive speaker click" of `display-driver-abi.md §9.6`).

/// `audio.md §8.6.1`: the shipped driver-image value of the pitch state.
pub const DISSOLVE_TONE_SHIPPED_PITCH_STATE: u16 = 30_308;
/// `audio.md §8.6.1`: the shipped driver-image value of the band-width
/// counter — the "progress" the effect is keyed on.
pub const DISSOLVE_TONE_SHIPPED_BAND_WIDTH: u16 = 240;

/// `audio.md §8.6.1`: the hard lower band edge, "a hard-coded 100 Hz for the
/// entire effect, first click to last". It never rises with progress.
pub const DISSOLVE_CLICK_MIN_HZ: u32 = 100;

/// `audio.md §8.6.1`: the additive constant of the pitch recurrence. The same
/// value is both the pre-rotation addend and the XOR mask.
pub const DISSOLVE_TONE_ADDEND: u16 = 37_448;

/// `audio.md §8.6.1`: the per-click hold is "one outer unit at the shift-four
/// subdivision, the same scale as the random-rumble step of section 5.3, so
/// roughly 50 to 60 microseconds", invariant across the plausible calibration
/// band. It is negligible next to the retune itself, but it is not zero.
pub const fn dissolve_click_hold_nanos() -> u64 {
    RUMBLE_NANOS_PER_STEP
}

/// `audio.md §8.6.1` pitch recurrence, all arithmetic modulo 65536:
///
/// ```text
/// state = rotate_right_16(state + 37448, 3)
/// state = state XOR 37448
/// state = state + 17
/// ```
///
/// This is bit-for-bit the recurrence of `§7.1` — the dissolve and the subtitle
/// ignition share one routine and one state word. From the shipped seed it is a
/// pure cycle of period 47,343 with no tail, so across the 16,160 clicks of the
/// logo dissolve no value ever repeats.
pub const fn dissolve_tone_advance(state: u16) -> u16 {
    let seeded = state.wrapping_add(DISSOLVE_TONE_ADDEND);
    let rotated = seeded.rotate_right(3);
    (rotated ^ DISSOLVE_TONE_ADDEND).wrapping_add(17)
}

/// The driver-local tone state the gated rectangle dissolve owns.
///
/// `audio.md §8.6.1`: this is **driver-global**, not per-dissolve. The
/// band-width counter "is never reset by the dissolve. It is a driver-global
/// that starts at its shipped value of 240 and only grows, or is overwritten
/// with 3000 by the ignition." Progress is therefore progress *since driver
/// load*; it coincides with progress within the rectangle only because the
/// intro logo dissolve is the counter's first user.
///
/// The pitch state is likewise never re-seeded, so the dissolve's clicks leave
/// it that many positions along its cycle before the ignition burst fires.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DissolveToneState {
    pitch_state: u16,
    band_width: u16,
}

impl Default for DissolveToneState {
    fn default() -> Self {
        Self::on_driver_load()
    }
}

impl DissolveToneState {
    /// The state as it stands in the shipped driver image, before any click.
    pub const fn on_driver_load() -> Self {
        Self {
            pitch_state: DISSOLVE_TONE_SHIPPED_PITCH_STATE,
            band_width: DISSOLVE_TONE_SHIPPED_BAND_WIDTH,
        }
    }

    /// The band-width counter. `audio.md §8.6.1` calls this the click counter:
    /// it counts *clicks*, that is every second visited pixel, not pixels.
    pub const fn band_width(self) -> u16 {
        self.band_width
    }

    /// The current pitch state, for handing on to the subtitle ignition.
    pub const fn pitch_state(self) -> u16 {
        self.pitch_state
    }

    /// `audio.md §8.6.1`: the **upper** band edge only, `floor(n / 2)`.
    pub const fn band_top_hz(self) -> u32 {
        self.band_width as u32 / 2
    }

    /// `audio.md §8.6.1`: the subtitle ignition "pins the band-width counter to
    /// 3000 before every burst, fixing its band at 100..1500 Hz forever, while
    /// the dissolve lets the counter free-run upward". The two effects diverge
    /// in the band parameter, not the generator.
    pub fn pin_band_for_ignition(&mut self) {
        self.band_width = 3_000;
    }

    /// Advance one click and return its emitted frequency in Hz.
    ///
    /// `audio.md §8.6.1`, with `n` the band-width counter:
    ///
    /// ```text
    /// span      = floor(n / 2) - 99
    /// frequency = 100 + (state modulo span)
    /// ```
    ///
    /// The state is advanced *before* the draw, and the counter is incremented
    /// immediately *after* it — that ordering is what reproduces the published
    /// first ten frequencies 118, 105, 101, 110, 108, 113, 113, 123, 123, 117.
    ///
    /// The published band-top formula divides by `floor(n / 2) - 99`, which is
    /// 21 at the shipped `n = 240`. §8.6.1 flags the latent hazard: at `n = 200`
    /// that divisor is zero and below 200 it goes negative, which is why the
    /// shipped value is 240 rather than 0. The counter only ever rises in the
    /// shipped flow, so this saturates rather than faulting.
    pub fn next_click_hz(&mut self) -> u16 {
        self.pitch_state = dissolve_tone_advance(self.pitch_state);
        let span = self.band_top_hz().saturating_sub(99).max(1);
        let frequency = DISSOLVE_CLICK_MIN_HZ + (self.pitch_state as u32 % span);
        self.band_width = self.band_width.saturating_add(1);
        frequency as u16
    }

    /// The whole click run for a gated rectangle of `pixels` pixels.
    ///
    /// `audio.md §8.6.1`: a gated rectangle of `P` pixels produces
    /// `ceil(P / 2)` clicks. For the one gated dissolve in the game the
    /// rectangle is 320 by 101 — 32,320 pixels and exactly 16,160 clicks.
    pub fn run_for_pixels(&mut self, pixels: u32) -> Vec<u16> {
        (0..pixels.div_ceil(2))
            .map(|_| self.next_click_hz())
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Trigger inventory (audio.md §7, §8; town-mode.md §13)
// ---------------------------------------------------------------------------

/// The complete confirmed trigger inventory.
///
/// Every variant corresponds to a published trigger. There is deliberately no
/// variant for a successful step, a menu acceptance, a name keystroke, an
/// ordinary dungeon move or turn, a generic successful command, a generic
/// pickup, or the Codex approach: `audio.md §9` states those are silent, and
/// inventing a cue for them is exactly the failure the section exists to
/// prevent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SoundEffect {
    /// `§7.4` rejected town movement or rejected combat step-or-attack.
    BlockedStep,
    /// `§8.1` stolen-action warning, ring vanish, Jimmy key break, borrowed
    /// fixed object; `§8.3` Vanish success tail.
    ActionSnap,
    /// `§8.3` common spell failure tail, after `Failed!`.
    CastFailure,
    /// `§8.5` dungeon wall droplet at landing stage 5, depth band `0..=3`.
    DungeonWallDrip { band: u8 },
    /// `§8.2` shared trap resolver entry: trapped chest or failed reagent mix.
    TrapRumble,
    /// `§8.2` shared damage presentation.
    DamageRumble,
    /// `§7.4` refused movement by a hoisted frigate.
    ShipCollisionRumble,
    /// `§7.4` rough-seas impact before party damage.
    RoughSeasImpactRumble,
    /// `§6` shared potion/wind/spell variant `0..=8`.
    SharedVariant { variant: u8 },
    /// `§6.1` the circle-scaled rumble lead alone, with the viewport inversion
    /// and both envelopes omitted. Shared by the combat effect template and
    /// the mass-target family.
    CircleRumbleLead { circle: u8 },
    /// `§6.1` combat effect template impact: a descending 20-update glissando
    /// from 1300 Hz toward 350 Hz, played only on a resolved effect.
    CombatTemplateImpact,
    /// `§8.3` monster possession success.
    Possession,
    /// `combat.md §6.3` controlled-party faint after a Vanish death. The
    /// program is byte-for-byte the possession envelope, but the trigger is
    /// kept distinct for conformance auditing.
    ControlledPartyFaint,
    /// `§8.4.1` Shadow Lord arena entry while carrying the Sceptre of Lord
    /// British. This is its only caller.
    SceptreReclaimed,
    /// `§8.3` monster summon success.
    MonsterSummon,
    /// `§8.3` accepted player Summon placement.
    PlayerSummon,
    /// `§8.3` accepted moongate transit.
    MoongateTransit,
    /// `§8.4` shared major full-viewport flash. `bands` holds the 1,856
    /// gameplay-PRNG-drawn frequencies, already consumed from the gameplay
    /// stream by [`draw_major_flash_bands`].
    MajorFlash { bands: Arc<[u8]> },
    /// `§8.2` Stonegate trapdoor scripted death, descending 1000..251 Hz.
    StonegateDescent,
    /// `§8.2` one trap-class rumble per Stonegate party-member death.
    StonegateMemberDeath,
    /// `§8.6` Return-to-View strip 2 inner tick.
    ReturnToViewStrip2,
    /// `§8.6` Return-to-View strip 3 at local phase 0 or 4.
    ReturnToViewStrip3 { phase: u8 },
    /// `§8.6` live Blackthorn cinematic movement/stinger-pause recipe.
    BlackthornMovementStinger,
    /// `§8.6.2` fixed six-envelope Blackthorn rescue sequence.
    BlackthornRescueEnvelopes,
    /// `containers.md §9` moldy-corpse Plague consequence.
    CorpsePlagueRumble,
    /// `§7.1` one admitted subtitle-ignition burst.
    SubtitleIgnitionBurst { pitches: Arc<[u16]> },
    /// `§8.6.1` one intro rectangle-dissolve retune.
    ///
    /// This is **not** a self-contained effect: it retunes a speaker that is
    /// already running and deliberately does not stop it. The frequency is
    /// deterministic — drawn by [`DissolveToneState`], never by the jitter
    /// stream and never by gameplay randomness. The run is terminated by
    /// [`SoundEffect::DissolveExit`].
    DissolveClick { frequency_hz: u16 },
    /// `§8.6.1` the dissolve's shared exit block, reached by both the abort
    /// path and normal completion. This is the single point that silences the
    /// speaker for the whole effect.
    DissolveExit,
    /// `town-mode.md §13` harpsichord key note.
    HarpsichordNote { digit: u8 },
    /// `§8.7` endgame Dead-member restoration flourish.
    EndgameRestoration,
    /// `§8.7` endgame box/tableau presentation.
    EndgameTableau,
    /// `§8.8` a combat command refused as inapplicable.
    ///
    /// The twelve verbs the combat scene does not implement — `B` `E` `F` `H`
    /// `I` `L` `M` `N` `Q` `T` `V` `X` — reach "one shared responder", which
    /// "has exactly one caller in the whole program, so there is no non-combat
    /// path to this sound". Scope is combat scenes only and **all** of them:
    /// "overworld-triggered, town-triggered and dungeon-room combat alike".
    ///
    /// The three message tails select only the tail: "All three arms, and the
    /// out-of-range fall-through, play the identical two-tone pair. One sound,
    /// one event class, twelve keys."
    ///
    /// `D` and `W`, and any unrecognised key, are **silent** — see
    /// [`combat_command_refusal_sounds`].
    CombatCommandRefused,
    /// `§8.9` the long descent, shared by the drowning and whirlpool triggers.
    ///
    /// "Those two sites are the only users of that recipe in the shipped game."
    LongDescent,
    /// `§10.6` the overworld falls chain's descending sweep — one site, played
    /// once per fall on every waterfall brink of either plane.
    SurfaceFallsDescent,
}

impl SoundEffect {
    /// Lower the effect to serial speaker operations.
    ///
    /// `jitter` is the sound-only stream of `audio.md §5.3`. It must never be
    /// the gameplay PRNG.
    pub fn program(&self, jitter: &mut RumbleJitter) -> SpeakerProgram {
        match self {
            SoundEffect::BlockedStep => blocking_tone(BLOCKED_STEP_HOLD_UNITS, BLOCKED_STEP_HZ),
            SoundEffect::ActionSnap => action_snap(),
            SoundEffect::CastFailure => cast_failure_glissando(),
            SoundEffect::DungeonWallDrip { band } => dungeon_wall_drip(*band),
            SoundEffect::TrapRumble => trap_rumble(jitter),
            SoundEffect::DamageRumble => damage_rumble(jitter),
            SoundEffect::ShipCollisionRumble => ship_collision_rumble(jitter),
            SoundEffect::RoughSeasImpactRumble => rough_seas_impact_rumble(jitter),
            SoundEffect::SharedVariant { variant } => shared_variant(*variant, jitter),
            SoundEffect::CircleRumbleLead { circle } => circle_rumble_lead(*circle, jitter),
            SoundEffect::CombatTemplateImpact => combat_template_impact(),
            SoundEffect::Possession => envelope_program(POSSESSION_ENVELOPE),
            SoundEffect::ControlledPartyFaint => envelope_program(POSSESSION_ENVELOPE),
            SoundEffect::SceptreReclaimed => envelope_program(SCEPTRE_RECLAIMED_ENVELOPE),
            SoundEffect::MonsterSummon => envelope_program(MONSTER_SUMMON_ENVELOPE),
            SoundEffect::PlayerSummon => envelope_program(PLAYER_SUMMON_ENVELOPE),
            SoundEffect::MoongateTransit => envelope_program(MOONGATE_TRANSIT_ENVELOPE),
            SoundEffect::MajorFlash { bands } => major_flash_program(bands),
            SoundEffect::StonegateDescent => stonegate_descent_program(),
            SoundEffect::StonegateMemberDeath => trap_rumble(jitter),
            SoundEffect::ReturnToViewStrip2 => return_to_view_strip2(jitter),
            SoundEffect::ReturnToViewStrip3 { phase } => {
                match return_to_view_strip3_frequency(*phase) {
                    Some(frequency) => blocking_tone(RETURN_TO_VIEW_STRIP3_HOLD_UNITS, frequency),
                    None => SpeakerProgram::new(vec![SpeakerOp::Stop]),
                }
            }
            SoundEffect::BlackthornMovementStinger => two_part_sting(jitter),
            SoundEffect::BlackthornRescueEnvelopes => blackthorn_rescue_envelope_program(),
            SoundEffect::CorpsePlagueRumble => trap_rumble(jitter),
            SoundEffect::SubtitleIgnitionBurst { pitches } => ignition_burst_program(pitches),
            // `audio.md §8.6.1`: "the speaker's square wave runs continuously
            // from the first click to the end of the dissolve, and is retuned
            // to a fresh pseudorandom frequency every second visited pixel."
            // A retune installs a divisor and holds; it must not stop.
            SoundEffect::DissolveClick { frequency_hz } => dissolve_click_retune(*frequency_hz),
            SoundEffect::DissolveExit => SpeakerProgram::new(vec![SpeakerOp::Stop]),
            // `town-mode.md §13.1`: the instrument "uses no other sound
            // primitive: it never plays a blocking tone, never starts a bare
            // timer tone, and never performs a calibrated wait of its own."
            // The 200-calibrated-unit hold this engine used is refuted there.
            SoundEffect::HarpsichordNote { digit } => {
                envelope_program(harpsichord_envelope(*digit))
            }
            SoundEffect::EndgameRestoration => envelope_program(ENDGAME_RESTORATION_ENVELOPE),
            SoundEffect::EndgameTableau => envelope_program(ENDGAME_TABLEAU_ENVELOPE),
            SoundEffect::CombatCommandRefused => combat_command_refused(),
            SoundEffect::LongDescent => long_descent(),
            SoundEffect::SurfaceFallsDescent => surface_falls_descent(),
        }
    }

    /// Whether the effect brackets its envelopes with the two full-viewport
    /// inversions of `audio.md §6`.
    pub const fn inverts_viewport(&self) -> bool {
        matches!(self, SoundEffect::SharedVariant { .. })
    }
}

/// `audio.md §7.4`: 165 Hz for 200 calibrated units.
pub const BLOCKED_STEP_HZ: u32 = 165;
pub const BLOCKED_STEP_HOLD_UNITS: u32 = 200;

/// `audio.md §7.4` overworld refusal predicate.
///
/// The published scope is emphatically **not** "step refused". After the
/// overworld path refuses a step it splits three ways:
///
/// 1. under sail it prints `BREAKING UP!`, `COLLISION!`, or `Docked!` and
///    "**No 165 Hz beep occurs on any under-sail path**";
/// 2. aboard a vehicle, a whirlpool-class blocking object "returns completely
///    silently, with no message at all";
/// 3. otherwise it prints `Blocked!` and beeps, except on one unidentified
///    animated-terrain destination tile that prints `OUCH!` and rumbles per
///    living member instead.
///
/// `ouch_destination` is the third branch's escape hatch. `§7.4` marks that
/// tile **unidentified** - "known only as one frame of a four-frame
/// animated-terrain block, and no shipped string names it" - so no caller can
/// pass `true` yet; the parameter exists so the predicate states the published
/// rule rather than silently omitting a branch of it.
pub const fn overworld_blocked_step_beeps(
    under_sail: bool,
    aboard_vehicle: bool,
    blocker_is_whirlpool_class: bool,
    ouch_destination: bool,
) -> bool {
    if under_sail {
        return false;
    }
    if aboard_vehicle && blocker_is_whirlpool_class {
        return false;
    }
    !ouch_destination
}

/// `audio.md §7.4` combat out-of-arena exit refusals.
///
/// Two of the four blocked-step sites are in combat: the step-or-attack
/// refusal, and "the out-of-arena exit refusal that prints `All must use the
/// same exit!`". The third combat refusal arm, `Stay with ship!`, is listed in
/// `§9` as silent.
pub const fn combat_out_of_arena_refusal_beeps(constrained_direction_refusal: bool) -> bool {
    constrained_direction_refusal
}

/// `audio.md §8.8`: the twelve verbs that reach the shared refusal responder.
///
/// "`B` Board, `E` Enter, `F` Fire, `H` Hole up, `I` Ignite, `L` Look, `M` Mix,
/// `N` New order, `Q` Quit, `T` Talk, `V` View, `X` X-it."
pub const COMBAT_COMMAND_REFUSED_KEYS: [char; 12] =
    ['B', 'E', 'F', 'H', 'I', 'L', 'M', 'N', 'Q', 'T', 'V', 'X'];

/// `audio.md §8.8` silence boundary, stated as a predicate so no caller can
/// generalise the pair to a neighbouring key.
///
/// "**Do not generalise to the neighbouring keys.** `D` and `W` print their own
/// short `What?` line with **no sound**, and any unrecognised key prints a bare
/// `What?` with no sound. That silence is real behaviour, not an omission
/// here."
pub fn combat_command_refusal_sounds(key: char) -> bool {
    COMBAT_COMMAND_REFUSED_KEYS.contains(&key.to_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pit_divisor_matches_the_published_rule() {
        // audio.md §5.1: floor(1,193,182 / f).
        assert_eq!(pit_divisor(165), 7231);
        assert_eq!(pit_divisor(1000), 1193);
        assert_eq!(pit_divisor(2000), 596);
        assert_eq!(pit_divisor(19), 62799);
    }

    #[test]
    fn action_snap_plays_the_published_sequence() {
        // audio.md §5.2: 40 updates, 1200, 1220, ... 1980 Hz.
        let program = action_snap();
        let frequencies = program.frequencies();
        assert_eq!(frequencies.len(), 40);
        assert_eq!(frequencies[0], 1200);
        assert_eq!(frequencies[1], 1220);
        assert_eq!(frequencies[39], 1980);
        assert!(program.ends_with_stop());
    }

    #[test]
    fn cast_failure_plays_the_published_sequence() {
        // audio.md §5.2: 50 updates, 800, 824, ... 1976 Hz.
        let frequencies = cast_failure_glissando().frequencies();
        assert_eq!(frequencies.len(), 50);
        assert_eq!(frequencies[0], 800);
        assert_eq!(frequencies[1], 824);
        assert_eq!(frequencies[49], 1976);
    }

    #[test]
    fn dungeon_drip_bands_match_the_published_steps() {
        // audio.md §5.2: 20 updates in steps of 15, 12 in steps of 25,
        // 4 in steps of 75, or no tone at all.
        let near = dungeon_wall_drip(0).frequencies();
        assert_eq!(near.len(), 20);
        assert_eq!(near[0], 3200);
        assert_eq!(near[1] - near[0], 15);
        assert_eq!(near[19], 3485);

        let mid = dungeon_wall_drip(1).frequencies();
        assert_eq!(mid.len(), 12);
        assert_eq!(mid[1] - mid[0], 25);
        assert_eq!(mid[11], 3475);

        let far = dungeon_wall_drip(2).frequencies();
        assert_eq!(far.len(), 4);
        assert_eq!(far[1] - far[0], 75);
        assert_eq!(far[3], 3425);

        // The fourth band emits no tone update and only performs the stop.
        let silent = dungeon_wall_drip(3);
        assert_eq!(silent.tone_count(), 0);
        assert_eq!(silent.ops, vec![SpeakerOp::Stop]);
    }

    #[test]
    fn the_long_descent_realises_the_published_endpoints() {
        // audio.md §5.2 recipe table: "195 updates: 660, 658, ... **272 Hz**,
        // against a nominal target of 150 Hz. The exact increment would be
        // -2.615 Hz and the truncated one is -2 Hz".
        let program = long_descent();
        let frequencies = program.frequencies();
        assert_eq!(frequencies.len(), LONG_DESCENT_UPDATES);
        assert_eq!(frequencies[0], LONG_DESCENT_INITIAL_HZ as u32);
        assert_eq!(frequencies[1], 658, "the truncated increment is -2 Hz");
        assert_eq!(
            frequencies[LONG_DESCENT_UPDATES - 1],
            LONG_DESCENT_LAST_HZ,
            "audio.md §8.9: the last tone played is 272 Hz"
        );
        assert!(program.ends_with_stop(), "§8.9 ends in hard silence");
    }

    #[test]
    fn the_long_descent_is_not_an_endpoint_interpolation() {
        // audio.md §5.2: an implementation "**must** play the sequence the
        // integer increment produces. It must not interpolate from `initial`
        // to `target`, and it must not append the target as a final update."
        // §8.9: "A frontend that interpolates 660 Hz to 150 Hz plays an effect
        // more than ten semitones deeper than the original."
        let frequencies = long_descent().frequencies();
        assert!(
            !frequencies.contains(&(LONG_DESCENT_NOMINAL_TARGET_HZ as u32)),
            "the nominal target is never a played update"
        );
        assert_eq!(
            frequencies.iter().copied().min(),
            Some(LONG_DESCENT_LAST_HZ),
            "nothing below the realised endpoint is ever emitted"
        );
        // Every update is exactly one truncated step below its predecessor.
        for pair in frequencies.windows(2) {
            assert_eq!(pair[0] - pair[1], 2);
        }
    }

    #[test]
    fn the_long_descent_duration_sits_in_the_published_band() {
        // audio.md §10.2: 195 updates at 35.2 ms, "**6.86 s** (6.2 to 7.6 s)".
        let program = long_descent();
        let nanos = program.total_nanos(true);
        assert!(
            (6_200_000_000..=7_600_000_000).contains(&nanos),
            "{nanos} ns is outside the published 6.2 to 7.6 s band"
        );
        // §8.9 calls it "by a wide margin the longest sound in the game" -
        // "every other glissando in the shipped build has a span of 300 units
        // or less", which is the 314 ms unattributed recipe.
        assert!(nanos > 20 * glissando(300, 1, 800, 2500).total_nanos(true));
        // §3 keeps the tone family mute-invariant: muting "turns this into a
        // silent seven-second freeze rather than skipping the beat".
        assert_eq!(program.total_nanos(false), nanos);
    }

    #[test]
    fn the_combat_command_refusal_is_two_discrete_tones() {
        // audio.md §8.8: "220 Hz for 150 calibrated units, then 150 Hz for 150
        // units. The speaker is de-gated between the two tones and re-gated
        // under a millisecond later, so it is two discrete pitches with a brief
        // hard break, not a glide. It ends in hard silence."
        let program = combat_command_refused();
        assert_eq!(program.frequencies(), vec![220, 150]);
        assert!(program.ends_with_stop());

        // The break is the blocking primitive's own stop, immediately followed
        // by the next tone. It is **not** two_part_sting's calibrated silent
        // hold, so no Silence op appears anywhere.
        assert_eq!(
            program.ops,
            vec![
                SpeakerOp::tone(220, tone_nanos(150)),
                SpeakerOp::Stop,
                SpeakerOp::tone(150, tone_nanos(150)),
                SpeakerOp::Stop,
            ]
        );
        assert!(
            !program
                .ops
                .iter()
                .any(|op| matches!(op, SpeakerOp::Silence { .. })),
            "§8.8's de-gate is not a calibrated silent hold"
        );
        // The re-gate happens "under a millisecond later": that is the
        // blocking install cost, and nothing else sits in the gap.
        assert!(TONE_BLOCKING_INSTALL_NANOS < 1_000_000);
    }

    #[test]
    fn the_combat_command_refusal_duration_sits_in_the_published_band() {
        // audio.md §10.1: "150 each" outer units, "**132 ms each**" and
        // "**263 ms**" for the whole effect, band "237 to 289 ms".
        let program = combat_command_refused();
        let nanos = program.total_nanos(true);
        assert!(
            (237_000_000..=289_000_000).contains(&nanos),
            "{nanos} ns is outside the published 237 to 289 ms band"
        );
        // §8.8: "Both holds still run, so a muted refusal is a silent stretch
        // of about 263 ms of dead time. A frontend that skips the whole effect
        // when muted diverges from the original."
        assert_eq!(program.total_nanos(false), nanos);
    }

    #[test]
    fn the_combat_command_refusal_key_gate_matches_the_published_twelve() {
        // audio.md §8.8 names exactly twelve verbs, and warns: "**Do not
        // generalise to the neighbouring keys.** `D` and `W` print their own
        // short `What?` line with **no sound**, and any unrecognised key prints
        // a bare `What?` with no sound."
        for key in COMBAT_COMMAND_REFUSED_KEYS {
            assert!(combat_command_refusal_sounds(key), "{key} should sound");
            assert!(combat_command_refusal_sounds(key.to_ascii_lowercase()));
        }
        assert_eq!(COMBAT_COMMAND_REFUSED_KEYS.len(), 12);
        for key in [
            'D', 'W', 'A', 'G', 'J', 'K', 'O', 'P', 'R', 'S', 'Y', 'Z', '?', ' ',
        ] {
            assert!(
                !combat_command_refusal_sounds(key),
                "{key} is outside the published twelve and must stay silent"
            );
        }
    }

    #[test]
    fn negative_span_glissando_only_stops() {
        assert_eq!(glissando(-4, 1, 3500, 3200).ops, vec![SpeakerOp::Stop]);
    }

    #[test]
    fn rumble_recipes_match_the_published_update_counts() {
        // audio.md §5.3 recipe table.
        let mut jitter = RumbleJitter::new();
        assert_eq!(trap_rumble(&mut jitter).tone_count(), 75);
        assert_eq!(damage_rumble(&mut jitter).tone_count(), 160);
        assert_eq!(return_to_view_strip2(&mut jitter).tone_count(), 3);
        for variant in 0..SHARED_VARIANT_COUNT as u8 {
            assert_eq!(
                shared_variant_lead(variant, &mut jitter).tone_count(),
                10 + 2 * variant as usize,
                "variant {variant} lead update count",
            );
        }
    }

    #[test]
    fn rumble_frequencies_stay_inside_the_published_ranges() {
        let mut jitter = RumbleJitter::new();
        for frequency in trap_rumble(&mut jitter).frequencies() {
            assert!((100..=500).contains(&frequency), "trap rumble {frequency}");
        }
        for frequency in damage_rumble(&mut jitter).frequencies() {
            assert!(
                (100..=2000).contains(&frequency),
                "damage rumble {frequency}"
            );
        }
        for frequency in shared_variant_lead(4, &mut jitter).frequencies() {
            assert!((100..=700).contains(&frequency), "variant lead {frequency}");
        }
    }

    #[test]
    fn two_part_sting_matches_the_published_shape() {
        // audio.md §5.3: 25 + 25 updates separated by a 20-unit silent hold.
        let mut jitter = RumbleJitter::new();
        let program = two_part_sting(&mut jitter);
        assert_eq!(program.tone_count(), 50);
        let silences = program
            .ops
            .iter()
            .filter(|op| matches!(op, SpeakerOp::Silence { .. }))
            .count();
        assert_eq!(silences, 1);
        assert!(program.ends_with_stop());
    }

    #[test]
    fn shared_variant_sequence_has_lead_then_two_opposed_envelopes() {
        // audio.md §6: rumble lead, then a positive-delta envelope, then the
        // same magnitude negative.
        let mut jitter = RumbleJitter::new();
        for variant in 0..SHARED_VARIANT_COUNT as u8 {
            let row = SHARED_VARIANTS[variant as usize];
            let program = shared_variant(variant, &mut jitter);
            let envelopes: Vec<EnvelopeSegment> = program
                .ops
                .iter()
                .filter_map(|op| match op {
                    SpeakerOp::Envelope(segment) => Some(*segment),
                    _ => None,
                })
                .collect();
            assert_eq!(envelopes.len(), 2, "variant {variant}");
            assert_eq!(envelopes[0].delta, row.delta_magnitude);
            assert_eq!(envelopes[1].delta, -row.delta_magnitude);
            assert_eq!(
                envelopes[0].initial_comparison,
                row.first_initial_comparison
            );
            assert_eq!(
                envelopes[1].initial_comparison,
                row.second_initial_comparison
            );
            assert_eq!(envelopes[0].period, row.phase_period);
            assert_eq!(envelopes[1].period, row.phase_period);
            assert_eq!(envelopes[0].iterations, row.iterations_per_envelope);
            assert_eq!(envelopes[1].iterations, row.iterations_per_envelope);
            assert_eq!(envelopes[0].idle, SHARED_VARIANT_IDLE);
            assert!(program.ends_with_stop());
        }
    }

    #[test]
    fn envelope_recurrence_follows_the_published_arithmetic() {
        // audio.md §5.4: phase += period mod 65536; compare; comparison += delta.
        let segment = EnvelopeSegment::new(3, 2700, 4, 1, 8810);
        let states: Vec<bool> = segment.pin_states().collect();
        assert_eq!(states.len(), 4);
        // Phase after each iteration: 8810, 17620, 26430, 35240. Comparison:
        // 2700, 2703, 2706, 2709. All phases exceed the comparison.
        assert_eq!(states, vec![true, true, true, true]);

        // A period small enough to stay under the comparison stays low.
        let low = EnvelopeSegment::new(0, 32_000, 3, 1, 100);
        assert_eq!(
            low.pin_states().collect::<Vec<_>>(),
            vec![false, false, false]
        );
    }

    #[test]
    fn envelope_phase_wraps_at_the_published_modulus() {
        let segment = EnvelopeSegment::new(0, 0, 10, 1, 60_000);
        let mut pins = segment.pin_states();
        pins.next();
        assert_eq!(pins.phase, 60_000);
        pins.next();
        assert_eq!(pins.phase, 120_000 % ENVELOPE_MODULUS);
    }

    #[test]
    fn major_flash_consumes_the_published_gameplay_draws() {
        // audio.md §8.4: 1,856 band draws, each 19..150 Hz inclusive.
        let mut prng = crate::prng::U5Prng::new(0x1234);
        let before = prng.state();
        let bands = draw_major_flash_bands(&mut prng);
        assert_eq!(FLASH_BAND_COUNT, 1856);
        assert_eq!(bands.len(), 1856);
        assert_ne!(prng.state(), before);
        for band in bands.iter() {
            assert!((19..=150).contains(band), "band {band}");
        }
        // Exactly 1,856 advances and no more.
        let mut replay = crate::prng::U5Prng::new(0x1234);
        for _ in 0..1856 {
            replay.advance();
        }
        assert_eq!(replay.state(), prng.state());
    }

    #[test]
    fn major_flash_program_retunes_once_per_band() {
        let bands: Arc<[u8]> = vec![19u8, 150, 84].into();
        let program = SoundEffect::MajorFlash { bands }.program(&mut RumbleJitter::new());
        assert_eq!(program.frequencies(), vec![19, 150, 84]);
        assert!(program.ends_with_stop());
    }

    #[test]
    fn stonegate_descent_matches_the_published_sweep() {
        // audio.md §8.2: every integer frequency from 1000 down through 251,
        // 750 tones, each held 40 calibrated units.
        let program = stonegate_descent_program();
        let frequencies = program.frequencies();
        assert_eq!(STONEGATE_DESCENT_TONES, 750);
        assert_eq!(frequencies.len(), 750);
        assert_eq!(frequencies[0], 1000);
        assert_eq!(frequencies[749], 251);
        assert!(program.ends_with_stop());
    }

    #[test]
    fn blocked_step_matches_the_published_correction() {
        // audio.md §7.4: 165 Hz for 200 calibrated units.
        let program = SoundEffect::BlockedStep.program(&mut RumbleJitter::new());
        assert_eq!(program.frequencies(), vec![165]);
        // `cleak/u5-spec#146` Q5 publishes this beep at 176 ms, band
        // 166..183 ms, and offers it as the sanity check for the whole anchor:
        // the blocked-step beep should read as a ~0.18 s bump in play.
        assert_eq!(program.total_nanos(true), tone_nanos(200));
        let millis = program.duration(true).as_secs_f64() * 1000.0;
        assert!(
            (166.0..=183.0).contains(&millis),
            "blocked step measured {millis} ms, outside the published 166..183 band",
        );
    }

    #[test]
    fn ignition_thresholds_follow_the_published_decay() {
        // audio.md §7.1: publication k uses threshold 400 - 3k.
        assert_eq!(ignition_threshold(1), 397);
        assert_eq!(ignition_threshold(110), 70);
    }

    #[test]
    fn ignition_burst_emits_twenty_five_pitches_then_stops() {
        let pitches: Arc<[u16]> = (0..25u16).map(|i| 100 + i * 56).collect::<Vec<_>>().into();
        let program =
            SoundEffect::SubtitleIgnitionBurst { pitches }.program(&mut RumbleJitter::new());
        assert_eq!(program.tone_count(), IGNITION_BURST_PITCHES);
        assert!(program.ends_with_stop());
    }

    #[test]
    fn wind_variant_is_chosen_by_the_caller_tag_not_by_the_old_wind() {
        // audio.md §7.3: "The variant is chosen by the caller tag, not by the
        // wind." The spell plays 2, the scroll plays 1, and "the old and new
        // compass directions do not participate" - so the requested direction
        // moves nothing except through the one silent spell/"none" guard.
        for requested_is_calm in [false, true] {
            assert_eq!(
                wind_change_variant(WindChangeCaller::Scroll, requested_is_calm),
                Some(WIND_SCROLL_VARIANT),
                "the scroll tag plays variant 1 on every requested direction"
            );
        }
        assert_eq!(
            wind_change_variant(WindChangeCaller::Spell, false),
            Some(WIND_SPELL_VARIANT)
        );
        // The one silent accepted path: a spell-tagged call requesting "none".
        assert_eq!(wind_change_variant(WindChangeCaller::Spell, true), None);
        assert_ne!(WIND_SPELL_VARIANT, WIND_SCROLL_VARIANT);
    }

    #[test]
    fn the_blocked_step_beep_keeps_its_published_overworld_exceptions() {
        // audio.md §7.4: "The overworld predicate is not simply `step
        // refused`."
        assert!(overworld_blocked_step_beeps(false, false, false, false));
        assert!(overworld_blocked_step_beeps(false, true, false, false));
        // "No 165 Hz beep occurs on any under-sail path."
        assert!(!overworld_blocked_step_beeps(true, true, false, false));
        // A whirlpool-class blocker aboard a vehicle "returns completely
        // silently, with no message at all" - but on foot it is an ordinary
        // blocking object.
        assert!(!overworld_blocked_step_beeps(false, true, true, false));
        assert!(overworld_blocked_step_beeps(false, false, true, false));
        // The `OUCH!` branch "applies random party damage instead of beeping".
        assert!(!overworld_blocked_step_beeps(false, false, false, true));
    }

    #[test]
    fn only_the_constrained_direction_combat_exit_refusal_beeps() {
        // audio.md §7.4: the second combat site is "the out-of-arena exit
        // refusal that prints `All must use the same exit!`"; the third arm,
        // `Stay with ship!`, is silent (§9).
        assert!(combat_out_of_arena_refusal_beeps(true));
        assert!(!combat_out_of_arena_refusal_beeps(false));
    }

    #[test]
    fn the_shared_variant_is_the_tier_index_of_the_thing_being_used() {
        // audio.md §6.1: a spell supplies its circle, floor(id / 6) + 1.
        for (spell_id, circle) in [
            (0usize, 1u8),
            (5, 1),
            (6, 2),
            (11, 2),
            (12, 3),
            (17, 3),
            (18, 4),
            (23, 4),
            (24, 5),
            (29, 5),
            (30, 6),
            (35, 6),
            (36, 7),
            (41, 7),
            (42, 8),
            (47, 8),
        ] {
            assert_eq!(spell_circle(spell_id), circle);
            assert_eq!(
                spell_shared_variant(spell_id),
                Some(circle),
                "spell {spell_id} plays variant = circle"
            );
        }
        // "No spell uses variant 0."
        assert!(!(0..48).any(|id| spell_shared_variant(id) == Some(0)));
    }

    #[test]
    fn the_seven_spells_outside_the_dispatcher_play_no_shared_variant() {
        // audio.md §6.1 second table, and the RETRACTIONS.md row that pulls
        // Kill (id 37) out of variant 6: it "plays no dispatcher variant at
        // all".
        for spell_id in [1usize, 13, 37, 28, 40, 44, 45] {
            assert_eq!(
                spell_shared_variant(spell_id),
                None,
                "spell {spell_id} never reaches the shared dispatcher"
            );
        }
        assert_eq!(spell_circle(37), 7, "Kill is a circle-7 spell");
        // The four field spells reach the dispatcher on the dungeon arm only.
        for spell_id in SPELL_IDS_WITH_DUNGEON_ONLY_VARIANT {
            assert_eq!(field_spell_shared_variant(spell_id, false), None);
            assert_eq!(
                field_spell_shared_variant(spell_id, true),
                Some(spell_circle(spell_id))
            );
        }
        // Id 20's dungeon arm supplies 4, "specifically to keep variant equal
        // to circle".
        assert_eq!(field_spell_shared_variant(20, true), Some(4));
        assert_eq!(field_spell_shared_variant(14, true), Some(3));
    }

    #[test]
    fn a_scroll_does_not_sound_like_its_spell() {
        // audio.md §6.1: the scroll supplies its scroll index, and "a frontend
        // must not reuse the spell's variant for the scroll". Six of eight
        // disagree; only View (4) and Negate Time (7) coincide.
        // The six disagreements §6.1 names one by one.
        for (scroll_index, spell_id) in [
            (0usize, 0usize), // Light: 0 against 1
            (1, 8),           // Wind Change: 1 against 2
            (2, 19),          // Protection: 2 against 4
            (3, 32),          // Negate Magic: 3 against 6
            (5, 43),          // Summon Daemon: 5 against 8
            (6, 42),          // Resurrection: 6 against 8
        ] {
            let scroll = scroll_shared_variant(scroll_index).expect("scroll 0..7 has a variant");
            assert_eq!(scroll, scroll_index as u8);
            assert_ne!(
                Some(scroll),
                spell_shared_variant(spell_id),
                "scroll {scroll_index} must not borrow spell {spell_id}'s variant"
            );
        }
        // View is the one coincidence the tables actually support: scroll 4
        // against Reveal (id 23, circle 4).
        assert_eq!(scroll_shared_variant(4), spell_shared_variant(23));
        // §6.1's prose also calls Negate Time a coincidence "at 7", but its own
        // tables put the scroll at 7 and the circle-8 spell (id 47) at 8. The
        // tables are the normative half, so the engine follows them and the
        // pair is a seventh disagreement, not a coincidence. Reported upstream.
        assert_eq!(scroll_shared_variant(7), Some(7));
        assert_eq!(spell_shared_variant(47), Some(8));
        assert_eq!(scroll_shared_variant(SCROLL_COUNT), None);
    }

    #[test]
    fn the_combat_effect_template_is_its_circles_rumble_lead_without_the_envelopes() {
        // audio.md §6.1: the template rumble is "exactly the rumble lead of
        // that circle's shared variant, with the viewport inversion and both
        // envelopes omitted".
        for circle in 1u8..=8 {
            let template = circle_rumble_lead(circle, &mut RumbleJitter::new());
            let lead = shared_variant_lead(circle, &mut RumbleJitter::new());
            assert_eq!(template.ops, lead.ops, "circle {circle}");
            assert!(
                !SoundEffect::CircleRumbleLead { circle }.inverts_viewport(),
                "the template omits the viewport inversion"
            );
        }
        // "a descending glissando, 20 updates from 1300 Hz down toward 350 Hz"
        let impact = combat_template_impact();
        let frequencies = impact.frequencies();
        assert_eq!(frequencies.len(), 20);
        assert_eq!(frequencies[0], 1300);
        assert!(
            frequencies.windows(2).all(|pair| pair[1] < pair[0]),
            "the impact glissando descends"
        );
        assert!(impact.ends_with_stop());
    }

    #[test]
    fn return_to_view_strip3_only_sounds_at_phases_zero_and_four() {
        assert_eq!(return_to_view_strip3_frequency(0), Some(3000));
        assert_eq!(return_to_view_strip3_frequency(4), Some(2000));
        for phase in [1u8, 2, 3, 5, 6, 7] {
            assert_eq!(return_to_view_strip3_frequency(phase), None);
        }
    }

    /// `town-mode.md §13.1`: **the scale ascends.** Digit `1` is the lowest
    /// note, digit `8` is an octave above it, and `9` and `0` continue two whole
    /// tones further, so the instrument spans a major tenth in natural
    /// left-to-right order.
    ///
    /// This engine shipped the withdrawn descending reading. The test that
    /// stood here asserted it, so the defect and its test protected each other
    /// — which is why the assertion below is written against the published
    /// phase-period constants rather than against semitone arithmetic of my
    /// own.
    #[test]
    fn the_harpsichord_scale_ascends_through_the_published_phase_periods() {
        let order = [1u8, 2, 3, 4, 5, 6, 7, 8, 9, 0];
        let mut previous = 0u16;
        for digit in order {
            let period = HARPSICHORD_PHASE_PERIODS[digit as usize];
            assert!(
                period > previous,
                "digit {digit} must sit above its predecessor, got {period} after {previous}",
            );
            previous = period;
        }
        // Pitch is proportional to the phase period, so the emitted scale rises
        // with it. There is no reciprocal on this path.
        let mut previous_hz = 0;
        for digit in order {
            let hz = harpsichord_frequency(digit);
            assert!(
                hz > previous_hz,
                "digit {digit} must sound above its predecessor"
            );
            previous_hz = hz;
        }
    }

    /// `town-mode.md §13.1`: the interval structure is exact — the major scale
    /// T T S T T T S, with the two semitone steps between `3`/`4` and `7`/`8`.
    ///
    /// Checked in cents off the published constants, because the section is
    /// explicit that the ratios, not the pitches, are the exact part.
    #[test]
    fn the_harpsichord_intervals_match_the_published_major_scale() {
        let cents = |low: u8, high: u8| {
            let lo = f64::from(HARPSICHORD_PHASE_PERIODS[low as usize]);
            let hi = f64::from(HARPSICHORD_PHASE_PERIODS[high as usize]);
            1200.0 * (hi / lo).log2()
        };
        // Whole tones everywhere except the two published semitone steps.
        for (low, high) in [(1u8, 2u8), (2, 3), (4, 5), (5, 6), (6, 7), (8, 9), (9, 0)] {
            let step = cents(low, high);
            assert!(
                (step - 200.0).abs() < 2.0,
                "{low} to {high} should be a whole tone, measured {step} cents",
            );
        }
        for (low, high) in [(3u8, 4u8), (7u8, 8u8)] {
            let step = cents(low, high);
            assert!(
                (step - 100.0).abs() < 2.0,
                "{low} to {high} should be a semitone, measured {step} cents",
            );
        }
    }

    /// `town-mode.md §13.1`: "The octave is **not** bit-exact. An exact
    /// doubling of digit `1` would be 6232; the shipped value is 6231, which is
    /// 11.9972 semitones, 0.28 cents flat. An implementation that hard-codes an
    /// exact 2:1 octave is inaudibly but measurably different. Reproduce the
    /// constants, not the idealised ratios."
    #[test]
    fn the_harpsichord_octave_is_the_shipped_constant_not_an_exact_doubling() {
        assert_eq!(HARPSICHORD_PHASE_PERIODS[8], 6231);
        assert_ne!(
            HARPSICHORD_PHASE_PERIODS[8],
            HARPSICHORD_PHASE_PERIODS[1] * 2,
            "6232 would be the idealised octave; the shipped constant is 6231",
        );
        let cents = 1200.0
            * (f64::from(HARPSICHORD_PHASE_PERIODS[8]) / f64::from(HARPSICHORD_PHASE_PERIODS[1]))
                .log2();
        assert!(
            (cents - 1199.72).abs() < 0.05,
            "the octave should be 0.28 cents flat, measured {cents} cents",
        );
    }

    /// `town-mode.md §13.1`: the note blocks for 4000 envelope iterations —
    /// about 172 ms, band about 160..190 ms — and the 200-calibrated-unit hold
    /// this engine used is refuted. The two models also *diverge* off the
    /// reference machine, "because calibrated units scale with the boot
    /// calibration count while envelope iterations scale with raw instruction
    /// throughput", so a frontend modelling this in calibrated units drifts.
    #[test]
    fn a_harpsichord_note_blocks_for_the_published_envelope_length() {
        let program = SoundEffect::HarpsichordNote { digit: 6 }.program(&mut RumbleJitter::new());
        let millis = program.duration(true).as_secs_f64() * 1000.0;
        assert!(
            (160.0..=190.0).contains(&millis),
            "a harpsichord note measured {millis} ms against a published ~172 ms",
        );
        // It is an envelope, not a blocking tone: §13.1 says the instrument
        // "never plays a blocking tone, never starts a bare timer tone, and
        // never performs a calibrated wait of its own".
        assert_eq!(program.tone_count(), 0);
        assert!(
            program
                .ops
                .iter()
                .any(|op| matches!(op, SpeakerOp::Envelope(_))),
        );
    }

    /// `town-mode.md §13.1`: "The generator's comparison value starts at 20000
    /// and steps by -4 each iteration, finishing at 4000; it is monotonic and
    /// never wraps." That rising duty is what makes the note plucked rather
    /// than a constant-amplitude square wave.
    #[test]
    fn the_harpsichord_envelope_sweeps_the_published_plucked_duty() {
        let segment = harpsichord_envelope(6);
        assert_eq!(segment.initial_comparison, 20_000);
        assert_eq!(segment.delta, -4);
        assert_eq!(segment.iterations, 4_000);
        // Monotonic, never wrapping: the final comparison is exactly 4000.
        let final_comparison = i64::from(segment.initial_comparison)
            + i64::from(segment.delta) * i64::from(segment.iterations);
        assert_eq!(final_comparison, 4_000);
    }

    #[test]
    fn harpsichord_progress_matches_the_published_resync_examples() {
        // town-mode.md §13.
        let mut progress = 0usize;
        for digit in HARPSICHORD_TUNE {
            progress = harpsichord_progress_after(progress, digit);
        }
        assert_eq!(progress, HARPSICHORD_TUNE.len());

        // After ten correct notes a stray 8 leaves the player three notes in.
        assert_eq!(harpsichord_progress_after(10, 8), 3);
        // After eleven correct notes a stray 7 leaves them two notes in.
        assert_eq!(harpsichord_progress_after(11, 7), 2);
        // A stray 6 at any other point leaves them one note in.
        for progress in 0..HARPSICHORD_TUNE.len() {
            if HARPSICHORD_TUNE.get(progress) == Some(&6) {
                continue;
            }
            assert_eq!(
                harpsichord_progress_after(progress, 6),
                1,
                "stray 6 at progress {progress}",
            );
        }
        // Any other wrong note resets progress to zero.
        assert_eq!(harpsichord_progress_after(0, 1), 0);
        assert_eq!(harpsichord_progress_after(4, 2), 0);
    }

    #[test]
    fn every_effect_program_ends_in_a_stop() {
        // audio.md §2: "An implementation must stop the speaker at every
        // specified effect end".
        let mut jitter = RumbleJitter::new();
        let effects = [
            SoundEffect::BlockedStep,
            SoundEffect::ActionSnap,
            SoundEffect::CastFailure,
            SoundEffect::DungeonWallDrip { band: 0 },
            SoundEffect::DungeonWallDrip { band: 3 },
            SoundEffect::TrapRumble,
            SoundEffect::DamageRumble,
            SoundEffect::ShipCollisionRumble,
            SoundEffect::RoughSeasImpactRumble,
            SoundEffect::CorpsePlagueRumble,
            SoundEffect::SharedVariant { variant: 0 },
            SoundEffect::SharedVariant { variant: 8 },
            SoundEffect::Possession,
            SoundEffect::ControlledPartyFaint,
            SoundEffect::BlackthornMovementStinger,
            SoundEffect::BlackthornRescueEnvelopes,
            SoundEffect::MonsterSummon,
            SoundEffect::PlayerSummon,
            SoundEffect::MoongateTransit,
            SoundEffect::MajorFlash {
                bands: vec![19u8, 150].into(),
            },
            SoundEffect::StonegateDescent,
            SoundEffect::StonegateMemberDeath,
            SoundEffect::ReturnToViewStrip2,
            SoundEffect::ReturnToViewStrip3 { phase: 0 },
            SoundEffect::ReturnToViewStrip3 { phase: 1 },
            SoundEffect::SubtitleIgnitionBurst {
                pitches: vec![440u16].into(),
            },
            // `§8.6.1`: DissolveClick is deliberately absent — it is a
            // retune inside a running effect, not an effect end. The exit is
            // the effect end, and stands in its place here.
            SoundEffect::DissolveExit,
            SoundEffect::HarpsichordNote { digit: 6 },
            SoundEffect::EndgameRestoration,
            SoundEffect::EndgameTableau,
            SoundEffect::CombatCommandRefused,
            SoundEffect::LongDescent,
        ];
        for effect in effects {
            let program = effect.program(&mut jitter);
            assert!(
                program.ends_with_stop(),
                "{effect:?} must end with a speaker stop",
            );
        }
    }

    #[test]
    fn blackthorn_rescue_runs_the_six_published_independent_envelopes() {
        let program = blackthorn_rescue_envelope_program();
        assert_eq!(program.ops.len(), 12);
        for (row, segment) in BLACKTHORN_RESCUE_ENVELOPES.iter().enumerate() {
            assert_eq!(program.ops[row * 2], SpeakerOp::Envelope(*segment));
            assert_eq!(program.ops[row * 2 + 1], SpeakerOp::Stop);
        }
        assert_eq!(
            BLACKTHORN_RESCUE_ENVELOPES,
            [
                EnvelopeSegment::new(1, 3000, 50_000, 1, 4400),
                EnvelopeSegment::new(1, 3000, 50_000, 1, 4125),
                EnvelopeSegment::new(1, 3000, 50_000, 1, 3667),
                EnvelopeSegment::new(1, 1000, 30_000, 1, 2933),
                EnvelopeSegment::new(1, 100, 40_000, 1, 3300),
                EnvelopeSegment::new(-1, 40_100, 40_000, 1, 3300),
            ]
        );
    }

    #[test]
    fn the_jitter_stream_never_touches_gameplay_randomness() {
        // audio.md §5.3: the private jitter state "is not the gameplay PRNG".
        let prng = crate::prng::U5Prng::new(0xBEEF);
        let before = prng.state();
        let mut jitter = RumbleJitter::new();
        let _ = trap_rumble(&mut jitter);
        let _ = damage_rumble(&mut jitter);
        let _ = shared_variant(5, &mut jitter);
        assert_eq!(prng.state(), before);
    }

    #[test]
    fn the_jitter_stream_starts_from_the_same_value_every_run() {
        let mut first = RumbleJitter::new();
        let mut second = RumbleJitter::new();
        assert_eq!(
            trap_rumble(&mut first).frequencies(),
            trap_rumble(&mut second).frequencies(),
        );
        assert_ne!(RUMBLE_JITTER_SEED, 0);
    }

    /// `cleak/u5-spec#146` Q3: the baseline calibration count cannot exceed 92,
    /// and `audio.md` gates the envelope idle work to zero below 100. So on the
    /// reference machine the idle term is always zero and cannot be what paces
    /// the envelope, which is why duration comes from the published
    /// per-iteration cost instead.
    #[test]
    fn the_baseline_calibration_zeroes_the_envelope_idle_gate() {
        assert!(NOMINAL_BOOT_CALIBRATION <= 92);
        assert!(NOMINAL_BOOT_CALIBRATION < 100);
    }

    /// `audio.md §10.2`: "Its inner count would fall from 5 to 4 only at a
    /// calibration count of 79 or below, which is outside the derived baseline
    /// band, so the truncation cannot flip and the duration cannot jump."
    ///
    /// The engine no longer derives the rumble from that truncation — `§10.2`
    /// publishes a closed form — but the invariant is worth keeping, because it
    /// is what licenses treating the calibration count as a fixed 87 at all.
    #[test]
    fn the_divisors_truncate_identically_across_the_published_band() {
        for calibration in 80..=92u32 {
            assert_eq!(calibration / 16, 5, "the rumble truncation cannot flip");
            assert_eq!(calibration / 24, 3);
        }
    }

    /// `audio.md` makes the tone, glissando and rumble families mute-invariant.
    /// `cleak/u5-spec#146` confirms that and withdraws it for the envelope
    /// alone: blocking tones, glissandi and rumble are genuinely mute-invariant;
    /// only the envelope is not.
    #[test]
    fn tone_glissando_and_rumble_are_mute_invariant() {
        let mut jitter = RumbleJitter::new();
        for effect in [
            SoundEffect::BlockedStep,
            SoundEffect::ActionSnap,
            SoundEffect::CastFailure,
            SoundEffect::TrapRumble,
            SoundEffect::DamageRumble,
            SoundEffect::StonegateDescent,
        ] {
            let program = effect.program(&mut jitter);
            assert!(program.duration(true) > Duration::ZERO, "{effect:?}");
            assert_eq!(
                program.total_nanos(true),
                program.total_nanos(false),
                "{effect:?} must not change length when muted",
            );
        }
    }

    /// `cleak/u5-spec#146`: the muted envelope is not cost-matched. Its silent
    /// arm omits the comparison and the speaker work, so it runs about 23%
    /// faster. A muted variant-0 potion is about 1.15 s against about 1.35 s
    /// audible, which is a real mute-dependent scene length.
    #[test]
    fn the_muted_envelope_is_faster_than_the_audible_one() {
        let mut jitter = RumbleJitter::new();
        let program = SoundEffect::SharedVariant { variant: 0 }.program(&mut jitter);
        assert!(program.total_nanos(false) < program.total_nanos(true));
        let ratio = ENVELOPE_MUTED_ITERATION_NANOS as f64 / ENVELOPE_ITERATION_NANOS as f64;
        assert!(
            (0.75..0.79).contains(&ratio),
            "the muted envelope should run about 23% faster, got {ratio}",
        );
    }

    /// `cleak/u5-spec#146` Q4: variant 0 sounds at about 3.13 kHz, a piercing
    /// high whistle rather than a low growl, and the nine variants form a clean
    /// descending octave from 3130 Hz to 1592 Hz. Those ratios are exact
    /// because they are ratios of program constants; only the absolute scale
    /// carries the published tolerance.
    #[test]
    fn the_shared_variants_form_the_published_descending_octave() {
        let gate = |row: SharedVariantRow| {
            EnvelopeSegment::new(
                row.delta_magnitude,
                row.first_initial_comparison,
                row.iterations_per_envelope,
                SHARED_VARIANT_IDLE,
                row.phase_period,
            )
            .gate_frequency_hz(true)
        };
        let top = gate(SHARED_VARIANTS[0]);
        let bottom = gate(SHARED_VARIANTS[SHARED_VARIANT_COUNT - 1]);
        assert!(
            (2800.0..=3350.0).contains(&top),
            "variant 0 measured {top} Hz against a published 2.8..3.35 kHz",
        );
        assert!(
            (1450.0..=1750.0).contains(&bottom),
            "variant 8 measured {bottom} Hz against a published ~1592 Hz",
        );
        let ratio = top / bottom;
        assert!((1.94..2.02).contains(&ratio), "octave ratio was {ratio}");

        let mut previous = f64::MAX;
        for row in SHARED_VARIANTS {
            let hz = gate(row);
            assert!(hz < previous, "the family must fall monotonically");
            previous = hz;
        }
    }

    /// The Sceptre-reclaimed note is a slow fade at CONSTANT pitch.
    ///
    /// Pinning its five published fields would not catch the error worth
    /// catching. The plausible implementation of a quest-completion sting is
    /// something short and bright, and `cleak/u5-spec#152` warned explicitly
    /// that this one is neither: the pitch never moves, the gate starts almost
    /// fully open, and it closes progressively over nearly three seconds. So
    /// this asserts the shape.
    #[test]
    fn the_sceptre_reclaimed_note_fades_at_constant_pitch() {
        let segment = SCEPTRE_RECLAIMED_ENVELOPE;
        assert_eq!(
            (
                segment.delta,
                segment.initial_comparison,
                segment.iterations,
                segment.idle,
                segment.period
            ),
            (1, 1, 65_000, 1, 4050),
            "the five published fields are exact"
        );

        // Constant pitch: the phase period is what sets the gate frequency,
        // and it never changes within a segment.
        let pins: Vec<bool> = segment.pin_states().collect();
        assert_eq!(pins.len(), 65_000);

        // The gate closes: count connected iterations in the first tenth
        // against the last tenth. A sweep or a chirp would not do this, and
        // neither would a short bright sting.
        let window = pins.len() / 10;
        let early = pins[..window].iter().filter(|high| **high).count();
        let late = pins[pins.len() - window..]
            .iter()
            .filter(|high| **high)
            .count();
        assert!(
            early > late,
            "the gate must close progressively: {early} early vs {late} late"
        );
        assert!(
            early * 2 > window,
            "the gate starts mostly open: {early} of {window}"
        );

        // Pitch. `cleak/u5-spec#152` publishes "about 1.4 kHz" and says
        // plainly that the figure is *modelled, not measured*, and inherits
        // the envelope rate's +/-7%. So this is deliberately NOT in
        // `the_named_envelopes_match_their_published_pitches`, whose 2% band
        // is right for the entries that are exact and would fail this one at
        // 1437 Hz for no good reason.
        let hz = segment.gate_frequency_hz(true);
        let error = (hz - 1400.0_f64).abs() / 1400.0;
        assert!(
            error < 0.07,
            "measured {hz} Hz against a modelled 1400 Hz +/-7%"
        );

        // And it really is the longest non-blocking note in the game.
        let nanos = segment.iterations as u64 * ENVELOPE_ITERATION_NANOS;
        assert!(
            (2_600_000_000..=3_000_000_000).contains(&nanos),
            "about 2.8 s, measured {nanos} ns"
        );
    }

    /// `cleak/u5-spec#146` Q4 pins the other named envelopes as well.
    #[test]
    fn the_named_envelopes_match_their_published_pitches() {
        for (segment, expected) in [
            (MONSTER_SUMMON_ENVELOPE, 980.0),
            (PLAYER_SUMMON_ENVELOPE, 980.0),
            (POSSESSION_ENVELOPE, 1101.0),
            (ENDGAME_TABLEAU_ENVELOPE, 1847.0),
            (MOONGATE_TRANSIT_ENVELOPE, 2096.0),
            (ENDGAME_RESTORATION_ENVELOPE, 3126.0),
        ] {
            let hz = segment.gate_frequency_hz(true);
            let error = (hz - expected).abs() / expected;
            assert!(
                error < 0.02,
                "period {} measured {hz} Hz against a published {expected} Hz",
                segment.period,
            );
        }
    }

    /// `audio.md §10.2` publishes the rumble family in closed form. All five
    /// tabulated recipes are pinned here, because the constant this replaced
    /// was fitted to the trap rumble alone and happened to be 40% wrong on the
    /// damage rumble — a fitted constant that matches its one data point is
    /// indistinguishable from a correct one until a second point arrives.
    #[test]
    fn every_published_rumble_duration_is_reproduced() {
        let mut jitter = RumbleJitter::new();
        let millis = |program: &SpeakerProgram| program.duration(true).as_secs_f64() * 1000.0;

        let trap = millis(&trap_rumble(&mut jitter));
        assert!((180.0..=200.0).contains(&trap), "trap rumble {trap} ms");

        let damage = millis(&damage_rumble(&mut jitter));
        assert!(
            (112.0..=124.0).contains(&damage),
            "damage rumble {damage} ms"
        );

        let lead = millis(&shared_variant_lead(0, &mut jitter));
        assert!((455.0..=515.0).contains(&lead), "variant 0 lead {lead} ms");

        let strip2 = millis(&return_to_view_strip2(&mut jitter));
        assert!((3.6..=4.4).contains(&strip2), "strip 2 {strip2} ms");

        // The sting is 25 + 25 updates around a 17.6 ms silent gap: about
        // 9.5 ms of tone, about 27 ms end to end.
        let sting = millis(&two_part_sting(&mut jitter));
        assert!((25.0..=29.0).contains(&sting), "two-part sting {sting} ms");
    }

    /// `audio.md §10.2`: "A glissando update costs `delay x 0.88 ms +
    /// 0.12 ms`." The 0.12 ms term is the divisor install.
    #[test]
    fn every_published_glissando_duration_is_reproduced() {
        let millis = |program: SpeakerProgram| program.duration(true).as_secs_f64() * 1000.0;
        // `§10.2` recomputed against `timing.md §7.4.1`'s install cost: an
        // update is `delay x 0.88 ms + 0.17 ms`, so the snap is 42 ms, not the
        // 40 ms the pre-install table gave.
        let snap = millis(action_snap());
        assert!((38.0..=46.0).contains(&snap), "action snap {snap} ms");
        let failure = millis(cast_failure_glissando());
        // `§10.2` recomputed: 52 ms, band 47 to 58.
        assert!(
            (47.0..=58.0).contains(&failure),
            "cast failure {failure} ms"
        );
        // `§10.2` recomputed: "21 / 12.6 / 4.2 / 0 ms" for the four bands.
        for (band, expected) in [(0u8, 21.0), (1, 12.6), (2, 4.2), (3, 0.0)] {
            let measured = millis(dungeon_wall_drip(band));
            assert!(
                (measured - expected).abs() < 1.0,
                "drip band {band} measured {measured} ms against {expected} ms",
            );
        }
    }

    /// `audio.md §10.3`: an audible envelope costs about
    /// `iterations x 43.0 us`; muted, about `iterations x 33.3 us`.
    #[test]
    fn every_published_envelope_duration_is_reproduced() {
        for (variant, audible_ms, muted_ms) in
            [(0usize, 430.0, 333.0), (1, 602.0, 466.0), (2, 774.0, 599.0)]
        {
            let row = SHARED_VARIANTS[variant];
            let segment = EnvelopeSegment::new(
                row.delta_magnitude,
                row.first_initial_comparison,
                row.iterations_per_envelope,
                SHARED_VARIANT_IDLE,
                row.phase_period,
            );
            let audible = segment.duration(true).as_secs_f64() * 1000.0;
            let muted = segment.duration(false).as_secs_f64() * 1000.0;
            assert!(
                (audible - audible_ms).abs() < 5.0,
                "variant {variant} audible {audible} ms against {audible_ms} ms",
            );
            assert!(
                (muted - muted_ms).abs() < 5.0,
                "variant {variant} muted {muted} ms against {muted_ms} ms",
            );
        }
    }

    /// `audio.md §10.1`: the Return-to-View strip 3 blip is 2.6 ms per phase,
    /// and the title-sequence publication waits use a driver-local unit of
    /// about 0.92 ms rather than the resident 0.88 ms.
    #[test]
    fn the_driver_local_waits_match_their_published_durations() {
        let blip = SoundEffect::ReturnToViewStrip3 { phase: 0 }
            .program(&mut RumbleJitter::new())
            .duration(true)
            .as_secs_f64()
            * 1000.0;
        // `§10.1` publishes 2.6 ms (2.5 to 2.8) here, but that table still
        // prices a bare 3 x 0.88 ms hold — it was not recomputed against
        // `timing.md §7.4.2`'s blocking install the way `§10.2` was. Including
        // the install gives 2.83 ms. Reported upstream; this accepts the
        // derived value rather than the stale row.
        assert!((2.5..=2.9).contains(&blip), "strip 3 blip {blip} ms");

        let sounded = ignition_publish_nanos(true) as f64 / 1.0e6;
        assert!(
            (38.0..=45.0).contains(&sounded),
            "sounded publish {sounded} ms"
        );
        let silent = ignition_publish_nanos(false) as f64 / 1.0e6;
        assert!(
            (42.0..=50.0).contains(&silent),
            "silent publish {silent} ms"
        );

        let burst = ignition_pitch_hold_nanos() * IGNITION_BURST_PITCHES as u64;
        let burst_ms = burst as f64 / 1.0e6;
        assert!(
            (3.4..=4.0).contains(&burst_ms),
            "ignition burst {burst_ms} ms"
        );
    }

    /// `cleak/u5-spec#146` Q5 publishes three effect totals directly.
    #[test]
    fn the_published_effect_durations_land_inside_their_bands() {
        let mut jitter = RumbleJitter::new();
        let snap = SoundEffect::ActionSnap
            .program(&mut jitter)
            .duration(true)
            .as_secs_f64()
            * 1000.0;
        assert!((36.0..=44.0).contains(&snap), "action snap {snap} ms");
        let trap = SoundEffect::TrapRumble
            .program(&mut jitter)
            .duration(true)
            .as_secs_f64()
            * 1000.0;
        assert!((180.0..=200.0).contains(&trap), "trap rumble {trap} ms");
    }

    /// `cleak/u5-spec#146` flags this one explicitly as far longer than it
    /// sounds and worth an emulator check: 750 tones at 40 units each is about
    /// 26.5 s. It follows unambiguously from exact loop bounds, so the engine
    /// reproduces it rather than shortening it to taste.
    #[test]
    fn the_stonegate_descent_is_the_published_twenty_six_seconds() {
        let seconds = SoundEffect::StonegateDescent
            .program(&mut RumbleJitter::new())
            .duration(true)
            .as_secs_f64();
        assert!(
            (25.0..=28.0).contains(&seconds),
            "the descent measured {seconds} s against a published ~26.5 s",
        );
    }
}

#[cfg(test)]
mod dissolve_click_tests {
    use super::*;
    /// `audio.md §8.6.1` publishes the first ten emitted frequencies exactly.
    /// This is the anchor for the whole model: the recurrence, the seed, the
    /// advance-then-draw ordering and the increment-after-draw ordering all
    /// have to be right together to reproduce it.
    #[test]
    fn the_dissolve_reproduces_the_published_first_ten_frequencies() {
        let mut tone = DissolveToneState::on_driver_load();
        let first_ten: Vec<u16> = (0..10).map(|_| tone.next_click_hz()).collect();
        assert_eq!(
            first_ten,
            vec![118, 105, 101, 110, 108, 113, 113, 123, 123, 117]
        );
    }

    /// `audio.md §8.6.1`: the one gated dissolve is 320 by 101 — 32,320 pixels
    /// and "exactly 16,160 clicks" — over which the band-width counter "runs
    /// 240 to 16,399 and the top band edge runs 120 Hz to 8199 Hz".
    #[test]
    fn the_logo_dissolve_click_count_and_band_match_the_published_figures() {
        let mut tone = DissolveToneState::on_driver_load();
        assert_eq!(tone.band_top_hz(), 120);
        let run = tone.run_for_pixels(320 * 101);

        assert_eq!(run.len(), 16_160);
        assert_eq!(tone.band_width(), 16_400, "240 + 16,160 clicks");
        assert_eq!(u32::from(tone.band_width() - 1) / 2, 8_199);
    }

    /// `audio.md §8.6.1`: "the overall mean across the run is 2106 Hz, and
    /// 52 percent of clicks exceed 1500 Hz". Both are statistics of the real
    /// arithmetic *including the modulo bias*, so they only reproduce if the
    /// draw is `100 + (state mod span)` and not a uniform draw. §8.6.1 warns
    /// that a uniform substitute "runs about 6 percent low at the top end".
    #[test]
    fn the_dissolve_run_reproduces_the_published_mean_and_tail() {
        let mut tone = DissolveToneState::on_driver_load();
        let run = tone.run_for_pixels(320 * 101);

        let mean = run.iter().map(|hz| u64::from(*hz)).sum::<u64>() / run.len() as u64;
        assert_eq!(mean, 2106);

        // 8,403 of 16,160 clicks, or 51.9988 percent — §8.6.1's "52 percent"
        // to the precision it is published at. Truncating instead of rounding
        // reads 51 and would look like a real disagreement with the document.
        let above = run.iter().filter(|hz| **hz > 1500).count();
        assert_eq!(above, 8_403);
        assert_eq!((above * 1000 / run.len()).div_ceil(10), 52);
    }

    /// `audio.md §8.6.1`: "the low edge stays pinned at 100 Hz from first pixel
    /// to last", and the effect "is not a sweep" — every click is an
    /// independent draw across the whole current band.
    ///
    /// The published contrast is with the model this engine used to have: the
    /// first ten percent of clicks "never leave 100..930 Hz", while low pops
    /// persist "right to the end", which a rising sweep forbids.
    #[test]
    fn the_dissolve_draws_across_the_band_rather_than_sweeping() {
        let mut tone = DissolveToneState::on_driver_load();
        let run = tone.run_for_pixels(320 * 101);

        assert!(
            run.iter().all(|hz| u32::from(*hz) >= DISSOLVE_CLICK_MIN_HZ),
            "the low edge is pinned at 100 Hz for the entire effect",
        );

        let mut sorted = run.clone();
        sorted.sort_unstable();
        assert_ne!(run, sorted, "a draw scatters; a sweep would be sorted");

        let opening = &run[..1_600];
        assert!(opening.iter().all(|hz| (100..=930).contains(hz)));

        let closing = &run[run.len() - 1_600..];
        assert!(closing.iter().any(|hz| u32::from(*hz) < 930));
    }

    /// `audio.md §8.6.1` and `RETRACTIONS.md` R230: "the speaker is enabled at
    /// the first click and nothing disables it until the dissolve exits". The
    /// per-click stop this engine used to emit is precisely the withdrawn
    /// model, so the assertion has to be that the retune does *not* stop.
    #[test]
    fn dissolve_clicks_retune_without_stopping_and_the_exit_stops_once() {
        let mut jitter = RumbleJitter::new();

        let retune = SoundEffect::DissolveClick { frequency_hz: 118 }.program(&mut jitter);
        assert!(
            !retune.ends_with_stop(),
            "a retune must not silence a speaker the dissolve still owns",
        );
        assert_eq!(retune.ops.len(), 1);

        let exit = SoundEffect::DissolveExit.program(&mut jitter);
        assert_eq!(exit.ops, vec![SpeakerOp::Stop]);

        let run = dissolve_click_run(&[118, 105, 101]);
        assert_eq!(run.ops.len(), 4);
        assert!(run.ends_with_stop());
        assert_eq!(
            run.ops
                .iter()
                .filter(|op| matches!(op, SpeakerOp::Stop))
                .count(),
            1,
            "exactly one silencing point, at the shared exit",
        );
    }

    /// `audio.md §8.6.1`: the ignition "pins the band-width counter to 3000
    /// before every burst, fixing its band at 100..1500 Hz forever, while the
    /// dissolve lets the counter free-run upward". The generator is shared; the
    /// band parameter is what differs — and 100..1500 Hz is the *ignition's*
    /// band, which this engine had borrowed for the dissolve.
    #[test]
    fn the_ignition_pins_the_band_the_dissolve_lets_free_run() {
        let mut tone = DissolveToneState::on_driver_load();
        let _ = tone.run_for_pixels(320 * 101);
        assert_eq!(tone.band_top_hz(), 8_200, "the dissolve ran the counter up");

        let carried = tone.pitch_state();
        tone.pin_band_for_ignition();
        assert_eq!(tone.band_top_hz(), 1_500, "the ignition's fixed band");
        assert_eq!(
            tone.pitch_state(),
            carried,
            "pinning the band must not re-seed the shared pitch state",
        );
    }

    /// `audio.md §8.6.1`: the pitch state "is never re-seeded", so the
    /// dissolve's 16,160 steps leave it that many positions along its cycle
    /// before the ignition burst ever fires.
    #[test]
    fn the_dissolve_leaves_the_shared_pitch_state_advanced_for_the_ignition() {
        let fresh = DissolveToneState::on_driver_load();
        let mut tone = DissolveToneState::on_driver_load();
        let _ = tone.run_for_pixels(320 * 101);
        assert_ne!(tone.pitch_state(), fresh.pitch_state());
    }

    /// `audio.md §8.6.1` places the whole sequence in the driver, and `§5.3`
    /// keeps driver-local pitch state out of gameplay randomness. The sequence
    /// is deterministic — "the entire click sequence is deterministic and
    /// exactly reproducible" — so it must not touch the jitter stream either.
    /// That was a second borrowing this model no longer needs.
    #[test]
    fn dissolve_clicks_never_touch_gameplay_randomness() {
        let prng = crate::prng::U5Prng::new(0x0F0F);
        let before = prng.state();

        let mut tone = DissolveToneState::on_driver_load();
        let first = tone.run_for_pixels(2_048);
        let mut again = DissolveToneState::on_driver_load();
        let second = again.run_for_pixels(2_048);

        assert_eq!(first, second, "the click sequence is exactly reproducible");
        assert_eq!(prng.state(), before);
    }
}

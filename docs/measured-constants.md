# Measured constants

Every engine constant or hold whose value was measured, fitted, bracketed or
calibrated against a capture rather than read off the specification, with
where its value now comes from. Consult this before touching a pacing value,
and add a row (or update one) whenever a value is measured or a spec answer
replaces a measurement.

## Why this list exists

Until 2026-10-26 the paired harness handed every `both` key to the engine
first and to DOSBox about 1.3 s later (logged 1.4 to 2.9 s), one window focus
at a time (`--key-sync serial`). Any hold measured **from a key press** on the
stock side therefore read about 1.3 s short against the engine, and a value
fitted to such a capture absorbed that skew. The shrine kneel hold is the
example: fitted at 72 ticks against the spec's 46 world steps, corrected in
`efc8be79`. A hold measured **between two stock-side events** (record to
record, start to end of one sound) is unaffected.

With `--key-sync sendevent` both sides get each key within about 3 ms, and
`--reaction-probe MS` records when each side's frame first reacted (about
40 ms resolution, `record.json` `reactions`). Re-measure with:

```
python3 qa/tools/paired_suite.py --engine-dir <worktree> --fixed-clock 12:00:00.00 \
  --key-sync sendevent --reaction-probe 1500 <scenario>
```

Run one paired capture at a time.

## Columns

- **Source**: `spec` - the value is the published one and the comment cites
  it; `measured` - still unpublished, the engine's own measurement;
  `derived` - computed from spec or measured rows above it; `engine` - a
  presentation choice, not a fidelity claim.
- **Anchor**: what the measurement was timed between. `key` means from a
  key press (skew-biased under serial keys); `stock events` means between
  two events on the stock side (skew-immune); `n/a` for published values.
- **Date**: when the value or its source was last checked.

Spec citations are to the `u5-spec` HEAD of the date given (`ac2c62a` for
2026-10-26).

## Pacing and timing

| Constant | File | Value | Source | Anchor / how measured | Date |
|---|---|---|---|---|---|
| `GAMEPLAY_WORLD_TICK_INTERVAL_SECS` | `u5-bevy/src/lib.rs` | 54.9254 ms | spec `timing.md §8.1` ("exactly 54.9254 ms"); the engine's 54.913 ms measurement (#179) is quoted there as the check | stock events (idle animation transitions, no key) | 2026-10-26 |
| `INTRO_FLOURISH_STEP_INTERVAL_SECS` | `u5-bevy/src/lib.rs` | 14 ms | spec `timing.md §5.1` | n/a | 2026-10-26 |
| `INTRO_SUBTITLE_IGNITION_UNIT_SECS` | `u5-bevy/src/lib.rs` | 0.88 ms | spec `timing.md §7`, `intro.md §5` | n/a | 2026-10-26 |
| `INTRO_SUBTITLE_IGNITION_FRESH_RUN_NANOS` | `u5-bevy/src/lib.rs` | 12.03 s | spec `intro.md §5` (DOSBox 0.74-3, 3000 cycles) | n/a | 2026-10-26 |
| `INTRO_SUBTITLE_IGNITION_PUBLICATION_WORK_NANOS` | `u5-bevy/src/lib.rs` | ~11.0 ms | derived from the spec rows above | n/a | 2026-10-26 |
| `INTRO_PRESENTS_HOLD_BIOS_TICKS` | `u5-bevy/src/lib.rs` | 18 | spec `timing.md §5.1` table, `intro.md §3` ("eighteen timer ticks") | n/a | 2026-10-26 |
| `INTRO_START_MENU_ARCHIVE_LOAD_BIOS_TICKS` | `u5-bevy/src/lib.rs` | 19 | spec `intro.md §3` (about 1.07 s at 3000 cycles) | n/a | 2026-10-26 |
| `INTRO_IGNITION_ABORT_MENU_HOLD_BIOS_TICKS` | `u5-bevy/src/lib.rs` | 31 | spec `intro.md §5` (about 1.70 s at 3000 cycles) | n/a | 2026-10-26 |
| `INTRO_PREVIEW_TICK_BIOS_TICKS` | `u5-bevy/src/lib.rs` | 2 | spec `location-dat.md §11`, `intro.md §5`; two engine measurements corroborate | stock events (attract demo, no key) | 2026-10-26 |
| `OUTER_CALIBRATED_UNIT_NANOS` | `u5-runtime/src/audio.rs` | 880 us | spec (#146, `timing.md §7`) | n/a | 2026-10-26 |
| `TONE_SWEEP_INSTALL_NANOS` | `u5-runtime/src/audio.rs` | 174 us | spec `timing.md §7.4.1` (replaced a fitted 12 inner units) | n/a | 2026-10-26 |
| `RUMBLE_NANOS_PER_STEP` | `u5-runtime/src/audio.rs` | 60.5 us | spec `audio.md §10.2` (replaced a fitted value) | n/a | 2026-10-26 |
| `DRIVER_LOCAL_UNIT_NANOS` | `u5-runtime/src/audio.rs` | 920 us | spec `audio.md §10.1` | n/a | 2026-10-26 |
| `ENVELOPE_ITERATION_NANOS` | `u5-runtime/src/audio.rs` | 43 us | spec (#146 Q4) | n/a | 2026-10-26 |
| `FLASH_BAND_NANOS` | `u5-runtime/src/audio.rs` | 463 us | spec `audio.md §8.4` (0.859 / 0.860 s per invocation, DOSBox Staging) over 1,856 bands; first measured by the engine on `word-of-power-audio` | stock events (one effect's audible span) | 2026-10-26 |
| `MAGIC_ABSORBED_ENVELOPE` | `u5-runtime/src/audio.rs` | (2, 1000, 28000, 1, 9800) | spec `audio.md §6.1` recipe; the measurement only showed it was missing | n/a | 2026-10-26 |
| `SHARD_SWELL_PERIOD` | `u5-runtime/src/audio.rs` | 2640 | spec `audio.md §5.4.7`; capture consistent | n/a | 2026-10-26 |
| `GATED_DISSOLVE_CLICK_HOLD_NANOS` | `u5-runtime/src/dissolve.rs` | 55 us | spec `timing.md §5.1` | n/a | 2026-10-26 |
| `GATED_START_MENU_DISSOLVE_3000_CYCLES_NANOS` | `u5-runtime/src/dissolve.rs` | 2.90 s | spec `timing.md §5.1` (DOSBox 0.74-3, 3000 cycles) | n/a | 2026-10-26 |
| `GATED_DISSOLVE_MEASURED_CLICK_PERIOD_NANOS` | `u5-runtime/src/dissolve.rs` | 179,455 ns | derived: 2.90 s / 16,160 published visits | n/a | 2026-10-26 |
| `BLACKTHORN_RESCUE_WAIT_ENVELOPE_SEQUENCE` | `u5-runtime/src/blackthorn.rs` | 212 | spec `blackthorn.md §7.1` (11.65 s slumber to shout, 3000 cycles) | n/a | 2026-10-26 |
| `BLACKTHORN_RESCUE_WAIT_PAIRED_FLASH` | `u5-runtime/src/blackthorn.rs` | 31 | spec `blackthorn.md §6.2` (1.67 to 1.72 s) | n/a | 2026-10-26 |
| `BLACKTHORN_RESCUE_WAIT_VIEWPORT_DISSOLVE` | `u5-runtime/src/blackthorn.rs` | 30 | spec `blackthorn.md §7.1` (1.65 s; replaced a bracketed 60) | n/a | 2026-10-26 |
| `BLACKTHORN_RESCUE_WAIT_CELL_REVEAL` (x3) | `u5-runtime/src/blackthorn.rs` | 17 | spec `blackthorn.md §6.2` (0.90, 0.935, 0.935 s) | n/a | 2026-10-26 |
| `BLACKTHORN_RESCUE_UNTIMED_BEFORE_DARKNESS` / `_AROUND_DISSOLVE` / `_BEFORE_ENVELOPES` | `u5-runtime/src/blackthorn.rs` | 1 / 2 / 2 | spec `blackthorn.md §7.1` "Measured Stonegate segments" (4.80 s fall end to envelopes) | n/a | 2026-10-26 |
| `BLACKTHORN_AUDIENCE_EXIT_BEAT_BIOS_TICKS` | `u5-runtime/src/blackthorn.rs` | 73 | spec `blackthorn.md §4.2` (about 4.02 s after the key; replaced a measured 110, then 72) | n/a | 2026-10-26 |
| `BLACKTHORN_CAPTIVE_CELL_HANDOFF_BIOS_TICKS` | `u5-runtime/src/blackthorn.rs` | 5 | spec `blackthorn.md §4.2` (about 0.27 s) | n/a | 2026-10-26 |
| `BLACKTHORN_CAPTURE_FIRST_LINE_MILLITICKS` | `u5-runtime/src/blackthorn.rs` | 2.185 ticks | spec `blackthorn.md §3` capture narration timeline step 1 ("+0.12 s" from the echo; u5-spec fa63a69, issue #304 Q6). Not normative | n/a | 2026-10-27 |
| `BLACKTHORN_CAPTURE_SETUP_MILLITICKS` | `u5-runtime/src/blackthorn.rs` | 0.546 tick | spec `blackthorn.md §3` timeline step 8 ("only CPU time (about 0.03 s)") | n/a | 2026-10-27 |
| `BLACKTHORN_THRONE_REVEAL_MILLITICKS` | `u5-runtime/src/blackthorn.rs` | 18.025 ticks | spec `blackthorn.md §3` timeline step 17 ("CPU-bound and measured about 0.99 s"; replaced a whole 18 ticks) | n/a | 2026-10-27 |
| `PRESENTATION_ROW_PRINT_MILLITICKS` / `_REPAINT_` / `_STING_` | `u5-runtime/src/blackthorn.rs` | 0.3 / 0.4 / 0.41 tick | spec `karma.md §7` "Measured stock timeline" and "Pacing rule" (u5-spec afd2249): row print about 0.3 tick, repaint 0.3 to 0.5, sting 20 to 25 ms. Used by `blackthorn_capture_release_milliticks` for the capture narration and the guard-release route; the narration's beats land within 0.12 s of every published second of `blackthorn.md §3` (test `spec304_q6_capture_pacing_lands_on_the_published_seconds`). Replaced whole-tick holds that reached the greeting about 0.3 s early | stock events: `bt-capture-pacing-20261027-015430` guards and reveal, `bt-audience-20261027-012906` `ask1` | 2026-10-27 |
| `stonegate_trapdoor_death_hold_bios_ticks` (fn) | `u5-runtime/src/stonegate_trapdoor.rs` | sum of the step's queued sounds | engine: ends with this engine's last rumble. Spec `blackthorn.md §7.1` gives the fall end at +27.52 s for three members on DOSBox 0.74 (R625); the DOSBox Staging capture runs about 26.8 s | stock events (first sweep tone to last rumble), audio | 2026-10-26 |
| `SHRINE_PRESENTATION_BIOS_TICKS_PER_WORLD_STEP` | `u5-runtime/src/commands.rs` | 1 | spec floor `karma.md §7` "Entry pacing"; re-measured with synced keys on `shrine-enter-pacing` (efc8be79) | key (re-measured, sendevent) | 2026-10-26 |
| `SHRINE_KNEEL_HOLD_BIOS_TICKS` | `u5-runtime/src/commands.rs` | 49 | spec `karma.md §7` "Pacing rule" and "Measured stock timeline" (u5-spec afd2249, issue #304 Q8): 46 published steps on a fixed tick lattice plus the published untimed costs (3.7-tick preamble, 0.7-tick kneeling-pose repaint) puts the Kneel record 48.7 ticks after `E`, nearest whole tick 49 (`shrine_entry_lattice_timeline`). Spec: echo to Kneel about 48.9 ticks, 48.4 to 49.4 from the keypress. Replaced a measured 49 (before that 46, the floor; before that a serial-fitted 72) | key: our own stock Kneel record 2653 to 2704 ms after `E` (mean 48.6 ticks) over 4 sendevent runs of `shrine-three-mantras` agrees with the spec | 2026-10-27 |
| `SHRINE_EXIT_PACING_HOLD_BIOS_TICKS` | `u5-runtime/src/commands.rs` | 61 | derived: spec 61 world steps x 1 tick (was a serial-fitted 88) | key (re-measured, sendevent) | 2026-10-26 |
| `SHRINE_VIRTUE_QUESTION_HOLD_WORLD_TICKS` | `u5-runtime/src/commands.rs` | 11 | spec `karma.md §7` "Pacing rule" (u5-spec afd2249, issue #304 Q8): the Kneel record's 2.5 ticks of work end the first of the ten requests on the third edge after the walk, the question on the twelfth (60 ticks after `E`), 11 after the 49-tick Kneel. Spec: about 11.5 ticks glyph to glyph. Replaced a measured 11 (before that 16) | stock events: Kneel record to question 598 to 658 ms (mean 11.4 ticks) over 4 synced-key runs of `shrine-three-mantras` agrees with the spec | 2026-10-27 |
| `SHRINE_MANTRA_FIRST_HOLD_BIOS_TICKS` | `u5-runtime/src/commands.rs` | 6 | derived: spec 6 world steps x 1 tick (`karma.md §7`, §12); re-measured (was 10, a ~1.6-a-step conversion) | key: stock reacts 401 to 438 ms after the virtue Return over 5 sendevent runs with `--reaction-probe 1500` (immediate keys 37 to 71 ms), about 6 ticks | 2026-10-26 |
| `SHRINE_MANTRA_SUBSEQUENT_HOLD_BIOS_TICKS` | `u5-runtime/src/commands.rs` | 12 | derived: spec 12 world steps x 1 tick; re-measured (was 19, a ~1.6-a-step conversion) | key: stock reacts 728 to 765 ms after each mantra Return over 5 sendevent runs (11 Returns), about 12 to 13 ticks; at 12 the engine reacts within 0 to 33 ms of it (was 0.40 s late) | 2026-10-26 |
| `ENDGAME_ABSORPTION_HOLD_BIOS_TICKS` | `u5-runtime/src/endgame.rs` | 33 | spec `endgame.md §5.2` (about 1.8 s at 3000 cycles; replaced a measured value) | n/a | 2026-10-26 |
| `ENDGAME_GREETING_HOLD_BIOS_TICKS` | `u5-runtime/src/endgame.rs` | 40 | spec `endgame.md §5.2` (forty paced iterations; replaced a timelapse reading) | n/a | 2026-10-26 |
| `ENDGAME_TABLEAU_STING_HOLD_BIOS_TICKS` | `u5-runtime/src/endgame.rs` | 5 | spec `audio.md §8.7` counts (2 + 3) | n/a | 2026-10-26 |
| `ENDGAME_TABLEAU_LORD_BRITISH_ARRIVAL_HOLD_BIOS_TICKS` | `u5-runtime/src/endgame.rs` | 40 | spec `audio.md §8.7`, `endgame.md §5.2` | n/a | 2026-10-26 |
| `StagedNarration::push_ticks_chained` / `CHAIN_HANDOFF_LIMIT_SECS` | `u5-runtime/src/narration.rs` | 0.25 s | engine: the handoff window is a presentation choice; the chaining fixed a measured 350 ms vs 280 ms sting spacing on `doom-endgame-audio` | stock events (sting to sting) | 2026-10-26 |
| `STATIC_TILE_ANIMATION_PERIOD_TICKS` | `u5-runtime/src/animation.rs` | 8 | spec `animation.md §6` | n/a | 2026-10-26 |
| `TEXT_HOLD_TIMED_PAUSE_PASSES` | `u5-runtime/src/text_hold.rs` | 28 | spec `conversation.md §7.3` | n/a | 2026-10-26 |
| `CODEX_PRESENTATION_KEY_STEPS_NO_ORDAINED` | `u5-runtime/src/miscmsg_io.rs` | 3 keys | spec `karma.md §8.2` (a count, confirmed on `codex-enter`) | n/a | 2026-10-26 |
| `MIN_OPENING_SPACE_WAIT_MS` | `u5-tui/tests/paired_scenarios.rs` | 15,000 ms | harness: chosen to land the title-skipping Space seconds past the logo dissolve (which ends 10 to 13 s in on either side); still past it with synced keys | scenario start | 2026-10-26 |

## Not timing (skew-immune)

These were measured from still frames or rendered audio, so key skew never
entered them. Listed so the inventory is complete.

| Constant | File | Value | Source | How | Date |
|---|---|---|---|---|---|
| `DISPLAY_PIXEL_ASPECT` | `u5-bevy/src/lib.rs` | 1.2 | measured; no spec states a display aspect | DOSBox Staging frame geometry | 2026-10-26 |
| `DUNGEON_MIRROR_SPAN` | `u5-runtime/src/dungeon_view.rs` | 191 | spec `dungeon-mode.md §6.3` (R394, after the engine's measurement, #199) | pixel diff at native scale | 2026-10-26 |
| `ACTIVE_OBJECT_VARIANT_TRANSITION_PROBABILITY` | `u5-runtime/src/visibility.rs` | 0.7508 | spec `visibility.md §8.3`; recapture rates 0.695 to 0.753 | per-tick transition counts | 2026-10-26 |
| `SPEAKER_VOLUME` | `u5-bevy/src/lib.rs` | 0.9 | engine: level-matched over the rendered suite | RMS / peak of engine renders | 2026-10-26 |
| `PROPORTIONAL_TEXT_LINE_HEIGHT` | `u5-bevy/src/lib.rs` | 9 px | spec-observed stride (#70) | frame geometry | 2026-10-26 |

## Published as untimed (deliberately not modelled)

| Span | Engine | Spec | Date |
|---|---|---|---|
| Journey Onward load path: menu key read to the first command-wait pass | No hold. The engine reaches its first idle pass about 67 ms after the key | `intro.md §7` "Timing of the load path" and `timing.md §8.2` (u5-spec 2ba0812, issue #306): "It is untimed CPU work plus file reads. An implementation must not add any wait to it, and an instant load is correct behaviour." The stock's first idle pass falls about 216 ms after the load border at DOSBox 3000 cycles (288 to 387 ms after the key; about 1.7 s more when the tile archive still has to load), which the spec publishes as emulator-only figures. So the engine runs about 4 to 6 idle draws ahead of a stock capture after a load (the ship-commands probe saw 5: Rough seas damage index 153 vs 148 at x-j 8270 ms). The spec allows a parity harness to model this as "an optional emulator-profile latency ... a test setting, not a game rule, and must never be expressed as ticks"; no such harness setting exists yet | 2026-10-27 |

String literals whose comments say **Measured** (refusals, prompts, row
spacing) are content and layout observations from still frames. They carry no
timing and are not listed row by row.

## Audit of 2026-10-26

48 rows: 43 pacing, 5 not timing. Of the 43 pacing rows, 39 now take a
published value or derive from one (the five shrine walk, exit and mantra
holds count here: published step counts at the published one tick a step,
re-measured with synced keys). Two were key-anchored, unpublished and
re-measured on 2026-10-26: the mantra holds, 10 to 6 and 19 to 12 ticks
(the walk and exit were re-measured earlier the same day, efc8be79). Four
remain the engine's own: the virtue-question hold (measured between stock
records), the trapdoor death hold (engine sound lengths), the narration
chain window and the harness's opening wait. Comments were brought up to
date where the spec has since published a measured value
(`GAMEPLAY_WORLD_TICK_INTERVAL_SECS`, `FLASH_BAND_NANOS`,
`DUNGEON_MIRROR_SPAN`) and a stale shrine note was removed.

Open findings:

- After the virtue answer's Return the engine's frame changed at once (it
  logged `:<answer>` and both feeds immediately), while the stock shows
  nothing for about 0.35 s, until the six-step wait ends and its newline
  prints. Fixed 2026-10-26: the answer row stays open through the hold and
  both feeds print when it releases (`karma.md §7`/§12).
- The same fine sampler put the stock Kneel record 2653 to 2704 ms after
  the `E` key over four runs, and this engine's (46 ticks) at 2522 to
  2543 ms, about 2.5 ticks early. `SHRINE_KNEEL_HOLD_BIOS_TICKS` became a
  measured 49. Since 2026-10-27 it and the question hold derive from
  `karma.md §7`'s "Pacing rule" (u5-spec afd2249, issue #304 Q8), which gives
  the same 49 and 11; the spec warns that fixed totals "are calibrated to
  that host's text speed; they are not program constants". The 61-step exit
  has not been measured this finely and stays at the floor.
- `SHRINE_VIRTUE_QUESTION_HOLD_WORLD_TICKS` was 16 and disagreed with the
  mantra holds' one tick a step. Re-measured on 2026-10-26 with a finer
  sampler (see its row): 11.

# Intro Graphics Gaps

Last refreshed: 2026-08-22 against `cleak/u5-spec` head `8192d67` and the
current `u5-engine` tree.

This file tracks the intro graphics contracts that still block an authentic
minimum intro implementation under the strict no-fallback rule. If a behavior is
listed here, `u5-engine` should fail loudly rather than render invented text,
generated animation, cropped buffers, or diagnostic substitutes.

## Still Open

The `cleak/u5-spec` queue is empty - 83 closed, 0 open - so nothing on the
intro path is waiting on a contract. What is listed here is engine work and the
handful of details the spec states honestly that it does not publish.

| Item | Status |
|---|---|
| Ultima IV transfer preview | **The one unimplemented published contract on this path.** `#73` is closed and `u4-transfer.md §6.1`-`§6.6` publish the per-field cursor cells, the label strip, the "Found:" summary page, the media-selection behaviour, the stage machine and the finish. The graphical preview has not been built against them, so `require_published_u4_transfer_preview_presentation` still fires when a *valid* `PARTY.SAV` is present. Missing or unreadable media is not gated - it takes the published `§3` retryable branch. `§6.4`'s "Please insert the Ultima IV Player Disk" block is statically orphaned in the shipped build and must not be drawn. |
| Acknowledgements wipe cadence | Published and implemented: 8-px stride, 18 steps, one BIOS tick per step on the part and close phases, no wait on rise and sink. `#72` is closed. |
| Rune digraph code points | `endgame.md §9.3` publishes that TH and ST each occupy one character and that the at-sign is the word space, but not which code points the digraphs use - and explicitly allows an engine to supply its own mapping. The engine applies the published word-space rule and leaves the digraphs as two runes. |

Four `panic!` sites on this path are structural rather than gaps: the terminal
shell refuses story slides, Return-to-View, acknowledgements and the transfer
preview because they are graphical screens and `--intro` has no surface to draw
them on, and the display driver's title-tick operation requires the caller to
inject the `ULTIMA` bands rather than generating clean-room frames.

## Answered Or Non-Blocking For This Pass

| Issue | Status |
|---:|---|
| [#52](https://github.com/cleak/u5-spec/issues/52) / [#65](https://github.com/cleak/u5-spec/issues/65) / [#63](https://github.com/cleak/u5-spec/issues/63) | Resolved by clean runtime observation of the shipped assets, pending the spec correction requested as [#78](https://github.com/cleak/u5-spec/issues/78). See "Resolved By Runtime Observation" below. |
| [#64](https://github.com/cleak/u5-spec/issues/64) | CREATE/chargen panel placements published and implemented. The paragraph rectangles `§5.1` never published were measured off a capture instead of invented - `CHARGEN_GYPSY_PARAGRAPH_BOX`, `CHARGEN_QUESTION_PARAGRAPH_BOX`, `CHARGEN_RESULT_PARAGRAPH_BOX` in `crates/u5-runtime/src/story_layout.rs`. The name/gender prompts use `§5.1`'s published cells and share one uncleared screen. |
| [#66](https://github.com/cleak/u5-spec/issues/66) / [#67](https://github.com/cleak/u5-spec/issues/67) | Title bitmap layering, flourish script, palette and final pre-menu frame are answered and implemented; see the reconciliation table below. |
| [#68](https://github.com/cleak/u5-spec/issues/68) / [#77](https://github.com/cleak/u5-spec/issues/77) | Intro animation cadence is published (`timing.md §5`) and implemented: the flourish is the calibrated animation-script entry at 14 ms per presentation step, the signature advances one 32-stroke chunk per BIOS user-tick, and a no-key menu poll pass costs two ticks. |
| [#69](https://github.com/cleak/u5-spec/issues/69) | Inline doorway text for story step 6 is published and rendered; all 21 story steps draw. |
| [#70](https://github.com/cleak/u5-spec/issues/70) | Proportional font metrics are resolved. Line stride is 9 rows (`u5_runtime::PROPORTIONAL_LINE_STRIDE`) and the story/chargen paragraph regions derive from art placement rather than hard-coded rectangles. |
| [#71](https://github.com/cleak/u5-spec/issues/71) | Withdrawn as a visual contract by spec commit `6f9132f`: the "initial title/rune text" phase is the non-visual pre-flourish preparation pass (`intro.md §3` step 2), implemented in `crates/u5-runtime/src/intro_preflourish.rs`. The former panic gate is gone. |
| [#54](https://github.com/cleak/u5-spec/issues/54) | Return-to-View preview geometry is published and implemented; the graphical preview renders and `--visual-frame-suite` now runs to completion instead of aborting on it. |

## Reconciled With Spec Head `8192d67`

The spec's 2026-08-22 pass answered the whole intro sequence and retracted
several earlier answers. The engine now follows head `8192d67`:

| Contract | Correction | Where |
|---|---|---|
| Title tick (`#65`) | The four bands are `ULTIMA` records 1..=4 — the black-box finding below was confirmed and the "driver-internal pixels / author your own frames" answers in `#52`/`#65` were withdrawn. The staging is: clear the hidden surface, draw records 1..=4 at `(16, 0)`, `(16, 50)`, `(16, 100)`, `(16, 150)`; each tick copies 49 rows at the **full 320-pixel width** from hidden row `50 * frame` to visible rows `65..=113`, then advances mod 4. The flanks are part of the rectangle, so the engine stages each record onto a background-filled 320-wide band rather than blitting 288 wide and clearing the margins. Record 4 contributes only its first 49 rows. | `TitleTickFrameSet`, `parse_ultima_title_tick_frames` |
| Flourish script (`#67`) | The published row-reveal table was wrong: the script has **eight row groups per frame, 56 total**, of which **seven reveal steps** are ever presented, plus **six erase steps** between consecutive frames — `7x7 + 6x6 = 85` presentation steps, not 67 groups. Each presentation repaints the frame's whole band at full 320-pixel width with the visible rows packed contiguously and centred (`floor(c/2)` blank above, `ceil(c/2)` below); even frames fill top-down, odd frames bottom-up and are therefore mirrored and shifted one row down. Frame 5 names source row 19 twice and never row 29, so row 29 stays blank — that quirk is part of the contract. | `TITLE_FLOURISH_REVEAL_SETS`, `title_flourish_step_state`, `blit_intro_title_flourish_frame_buffer` |
| Flourish cadence (`#77`) | The flourish is **not** BIOS-tick paced and is not driven by the title-tick helper: it is one call into the driver's calibrated animation-script entry. `timing.md §5.1` now publishes **14 ms per presentation step, ~1.2 s total**. The earlier "one title-tick call per row-reveal group / ~3.7 s" answer was withdrawn. | `INTRO_FLOURISH_STEP_INTERVAL_SECS`, `visual_intro_animation_interval` |
| Title palette (`#66`) | The flourish is palette index **9** (a blue-plus-intensity write mask); slots 7, 8, 9, `BRITISH.BIT` and the live pen strokes are index **15**. The two-colour result is deliberate. | `compose_intro_title_flourish_source_buffer`, `blit_intro_title_slots` |
| Final pre-menu frame (`#66` Q4) | The title sequence is a series of **whole-page publishes**, so the final frame contains exactly three things: slot 8 at `(152, 0)`, `BRITISH.BIT` at `(24, 66)` and slot 9 at `(104, 160)`. It does **not** still contain the slot-6 mark or the slot-7 "Presents" line — the whole-page publish that opens the attribution card cleared them. Only slot 7's own draw is a partial publish, of `(0, 140)..(319, 199)`, which is why the mark survives beneath it during the hold. | `IntroTitleCompositionPhase`, `visual_intro_presents_hold_buffer` |
| Reveal transitions (`#53`) | The one-pixel-column-per-title-tick sweep is withdrawn in full for both intro callers. Both are the driver's **rectangle dissolve**: one blocking call visiting every pixel exactly once in a deterministic pseudo-random order, hidden surface to visible page. There is no per-column schedule and no tick pacing. | `IntroDisplayBuffer::dissolve_rect_from` |
| Menu idle cadence | A no-key menu poll pass costs two DOS BIOS user-ticks (~110 ms), so the 200-pass Return-to-View timeout is **~22 s**, and the flame band advances once per pass rather than once per tick. | `INTRO_MENU_IDLE_POLL_BIOS_TICKS` |

Residuals the spec states honestly and the engine inherits: the 14 ms flourish
step is a derived target inside a 10.5-15.8 ms bracket, not a measured figure;
the dissolve is self-paced with no published wall-clock length for any
rectangle, so the engine completes it as the single blocking call it is rather
than inventing a rate; and the `.4`-depth conversion of the title-tick records
is published as geometry only.

## Resolved By Runtime Observation (pending `cleak/u5-spec#78`)

The intro menu screen no longer has an unpublished contract. Decoding the local
`ULTIMA.16` image directory and comparing it against a black-box capture of the
original running the same assets settled three contracts the spec has wrong; the
engine implements the observed behaviour and every affected code path carries a
`cleak/u5-spec#78` comment naming the pending correction.

| Contract | What the spec says | What the original does |
|---|---|---|
| Menu backing art (`#63`, `systems/intro.md §3`) | `STARTSC`, as a 16 + 288 + 16 by 137 composition. | `ULTIMA` slot 0, the 319x61 "Ultima V" logo, at `(0, 0)` over a surface cleared to index 0. `STARTSC.16` decodes to the *credits* artwork the Acknowledgements screen shows, so it stays loaded only for that subflow. |
| Title-tick frames (`#52`, `#65`, `systems/intro.md §5`, `systems/display-driver.md §8`) | Four `320x49` bands that exist only in the EGA driver's runtime back-buffer and cannot be read from an external art file; clean engines should author replacements. | `ULTIMA` slots 1..=4 (288x49, 288x49, 288x49, 288x50) are the four flaming "Warriors of Destiny" bands. The upper 49 rows of each are blitted at `(16, 65)`; columns 0..=15 and 304..=319 of the published `(0, 65)` 320x49 rectangle are cleared to index 0, so the destination is still overwritten opaquely. The mod-4 advance and cadence are unchanged. Frame 0 = slot 1 is assumed (the captures pin slots 2 and 3 only). |
| Acknowledgements art (`#72`, `systems/intro.md §11`) | "Loads its own graphics resource from the end-screen asset family". | `ENDSC.16` is a single blank 260x168 parchment; `STARTSC.16` is the credits artwork. Pressing `A` keeps the `ULTIMA` logo on rows 0..=60 and replaces rows 63..=199 with the STARTSC 16 + 288 + 16 by 137 composition, bottom-aligned. No text is drawn over it - the credits are part of the artwork - and any key restores the menu. Only the entry/exit wipe cadence is still unpublished. |
| Lower menu frame (`#63`, `systems/intro.md §6.1`) | A single-line rectangle of five reserved box-drawing glyphs in the bright foreground index. | The same rounded blue chrome the gameplay border uses: pixel rows 120..=199 filled with EGA index 1 (fill starts at column 5, 3, 2, 1, 1, then 0, mirrored right and at the bottom), a one-pixel index-15 rectangle (rules at y = 127 and y = 192 over x = 7..=312, verticals at x = 7 and x = 312 over y = 128..=191), a black interior, and two white-on-black captions over the border rows: `>Select:` + the `IBM.CH` glyph-8 cursor + `<` at cells 15..=24 of row 15, and `>Copyright 1988 Lord British<` at cells 5..=33 of row 24. |

Two smaller `§6.2` corrections landed with them: the six labels are the full
strings ("Create New Character", "Transfer from Ultima IV", "Ultima V
Introduction", "Return to the View"), not the spec's abbreviations, and the
published one leading / one trailing blank is now emitted from the render path
so the origin cell holds the blank. The menu is presented with row 0 ("Journey
Onward") in inverse video before any key is pressed, and the highlight also
responds to the arrow keys with Space/Enter activating the highlighted row.

The procedural sine-wave flame generator, the published-palette-cycle table, the
`TITLE_TICK_*.png` loader, and the dead `EGA.DRV` plane unpacker are all deleted:
the real source is an asset the engine can read at runtime, so none of them are
needed.

## Verification Notes

- The strict no-fallback rule still holds: where a contract is unpublished the
  engine refuses rather than rendering invented text, generated animation,
  cropped buffers, or diagnostic substitutes.
- When one of these issues is answered, remove only the corresponding panic and
  add focused tests that prove the published contract is being followed.
- Read contracts from the `cleak/u5-spec` GitHub issues. The local checkout at
  `C:\Projects\Rust\u5-clean\u5-spec` is read-only from this workspace and is
  stale at `9a898d1`, now many commits and several retractions behind spec head
  - the reconciliation table would look wrong if checked against it.
- The shipped palette is not stock EGA: index 6 is `(170, 170, 0)` dark yellow,
  not `(170, 85, 0)` brown, and it is the only index that differs. `STORY1`
  slide 0 is 25% index 6 and the `STARTSC` acknowledgements parchment 6%, so
  both changed hue when it was corrected. Nothing reprograms the palette after
  mode setup.
- Intro evidence in `--visual-frame-suite`: `intro-menu`, `intro-finished-menu`,
  `intro-story-00`..`intro-story-20`, `intro-return-to-view`,
  `intro-chargen-name-prompt`, `intro-chargen-gender-prompt`,
  `intro-chargen-gender-echo`. 187 PNGs total on 2026-08-22 at `d4fc579`.
- The suite used to render no menu window at all while the live path was
  correct, because it built its intro state through a parallel path. It now
  drives the real render path, so a defect of that shape cannot hide there
  again.

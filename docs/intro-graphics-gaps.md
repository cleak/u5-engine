# Intro Graphics Gaps

Last refreshed: 2026-08-24 against public `cleak/u5-spec` head `5b9445f` and
the current `u5-engine` tree.

This file tracks the intro graphics contracts that still block an authentic
minimum intro implementation under the strict no-fallback rule. If a behavior is
listed here, `u5-engine` should fail loudly rather than render invented text,
generated animation, cropped buffers, or diagnostic substitutes.

## Still Open

There is no open blocking intro question. Public issues `#118` and `#120` are
resolved by clean commits `12485b3` and `36780cb`; the subtitle ignition now
implements the exact two-pass countdown/tails, per-state polling order,
`0x3500` Galois vector, first-publication anchors, gate recurrence, 45/50 pacing
ratio, and published `48/53` plus `35/33` burst totals.

**Nothing on the intro path that is fully published remains unimplemented.**
The Ultima IV transfer preview was the last complete missing contract. What is
left below is the handful of optional details the spec explicitly permits an
implementation to choose.

| Item | Status |
|---|---|
| Ultima IV transfer preview | **Implemented** (`f3ecfd1`). `u4-transfer.md §6.1`-`§6.6` publish the per-field cursor cells, the label strip, the "Found:" summary page, the media-selection behaviour, the stage machine and the finish; `crates/u5-runtime/src/u4_transfer_preview.rs` holds them as data and `crates/u5-bevy/src/u4_transfer.rs` composites them. `§6` has no double buffering, no page swap and no deferred flush, so the screen is one persistent surface drawn once and edited in place — every keystroke is a short list of edits. Retractions honoured: an eight-row label column rather than an eight-column heading strip; exactly eight fields; the panels read ` Ultima IV ` and ` Ultima  V ` after the single-cell blanking write; and once the drive is selected no key aborts, so `Esc` is ignored at every prompt. `§6.4`'s "Please insert the Ultima IV Player Disk" block is statically orphaned in the shipped build and is not drawn. The gate `require_published_u4_transfer_preview_presentation` is gone. Missing or unreadable media is still not gated - it takes the published `§3` retryable branch. Frame-suite evidence: `intro-u4-transfer-found`, `intro-u4-transfer-panels`. |
| Acknowledgements | **Implemented** (`6db6135`) as the `§11.2` four-phase artwork animation: compose, rise, part, keypress, close, sink. Rise is 137 unpaced steps, part and close are 18 steps each at one BIOS tick per step, sink is 136 + 1 unpaced. Step 6 rebuilds the menu on the hidden surface with the Acknowledgements row inverse-video while the credits are still displayed. Step 8 is not a literal mirror of step 5 — its band offsets walk the pillars back to rows 144..=175, exactly where step 10 picks them up. `Esc` is an ordinary key here. Four new frame-suite cases: `intro-acknowledgements-risen`/`-parting`/`-credits`/`-closing`. |
| Rune digraph code points | `endgame.md §9.3` publishes that TH and ST each occupy one character and that the at-sign is the word space, but not which code points the digraphs use - and explicitly allows an engine to supply its own mapping. The engine applies the published word-space rule and leaves the digraphs as two runes. |

### Retraction note: the slab-wipe model

The acknowledgements "bottom-up entry wipe / top-down exit wipe with horizontal
slabs" model this engine carried is **withdrawn in full**. There are no
horizontal slabs in either direction. Any surviving "slab" language outside this
paragraph is wrong and should be deleted, not reconciled.

Five refusals on this path are structural rather than gaps: the terminal shell
refuses story slides, Return-to-View, acknowledgements and the transfer preview
because they are graphical screens and `--intro` has no surface to draw them on,
and the display driver's title-tick operation requires the caller to inject the
`ULTIMA` bands rather than generating clean-room frames. The terminal transfer
refusal stays by design even though the graphical preview now exists: a text
transcript of it would be the invented substitute the no-fallback rule forbids.

## Answered Or Non-Blocking For This Pass

| Issue | Status |
|---:|---|
| [#52](https://github.com/cleak/u5-spec/issues/52) / [#65](https://github.com/cleak/u5-spec/issues/65) / [#63](https://github.com/cleak/u5-spec/issues/63) | Resolved by clean runtime observation of the shipped assets; the spec correction requested as [#78](https://github.com/cleak/u5-spec/issues/78) has since been published and closed. See "Resolved By Runtime Observation" below. |
| [#64](https://github.com/cleak/u5-spec/issues/64) | CREATE/chargen panel placements published and implemented. The paragraph rectangles `§5.1` never published were measured off a capture instead of invented - `CHARGEN_GYPSY_PARAGRAPH_BOX`, `CHARGEN_QUESTION_PARAGRAPH_BOX`, `CHARGEN_RESULT_PARAGRAPH_BOX` in `crates/u5-runtime/src/story_layout.rs`. The name/gender prompts use `§5.1`'s published cells and share one uncleared screen. |
| [#66](https://github.com/cleak/u5-spec/issues/66) / [#67](https://github.com/cleak/u5-spec/issues/67) | Title bitmap layering, flourish script, palette and final pre-menu frame are answered and implemented; see the reconciliation table below. |
| [#68](https://github.com/cleak/u5-spec/issues/68) / [#77](https://github.com/cleak/u5-spec/issues/77) | Intro animation cadence is published (`timing.md §5`) and implemented: the flourish is the calibrated animation-script entry at 14 ms per presentation step, the signature advances one 32-stroke chunk per BIOS user-tick, and a no-key menu poll pass costs two ticks. |
| [#69](https://github.com/cleak/u5-spec/issues/69) | Inline doorway text for story step 6 is published and rendered; all 21 story steps draw. |
| [#70](https://github.com/cleak/u5-spec/issues/70) | Proportional font metrics are resolved. Line stride is 9 rows (`u5_runtime::PROPORTIONAL_LINE_STRIDE`) and the story/chargen paragraph regions derive from art placement rather than hard-coded rectangles. |
| [#71](https://github.com/cleak/u5-spec/issues/71) | Withdrawn as a visual contract by spec commit `6f9132f`: the "initial title/rune text" phase is the non-visual pre-flourish preparation pass (`intro.md §3` step 2), implemented in `crates/u5-runtime/src/intro_preflourish.rs`. The former panic gate is gone. |
| [#54](https://github.com/cleak/u5-spec/issues/54) | Return-to-View preview geometry is published and implemented; the graphical preview renders and `--visual-frame-suite` now runs to completion instead of aborting on it. |

## Reconciled With Spec Head

The spec's 2026-08-22 pass answered the whole intro sequence and retracted
several earlier answers, and head has moved on again since (`c00bf63`,
`38b0231`). The engine follows current head:

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

## Resolved By Runtime Observation (`cleak/u5-spec#78`, now answered)

The intro menu screen no longer has an unpublished contract. Decoding the local
`ULTIMA.16` image directory and comparing it against a black-box capture of the
original running the same assets settled three contracts the spec has wrong; the
engine implements the observed behaviour and every affected code path carries a
`cleak/u5-spec#78` comment naming the pending correction.

| Contract | What the spec says | What the original does |
|---|---|---|
| Menu backing art (`#63`, `systems/intro.md §3`) | `STARTSC`, as a 16 + 288 + 16 by 137 composition. | `ULTIMA` slot 0, the 319x61 "Ultima V" logo, at `(0, 0)` over a surface cleared to index 0. `STARTSC.16` decodes to the *credits* artwork the Acknowledgements screen shows, so it stays loaded only for that subflow. |
| Title-tick frames (`#52`, `#65`, `systems/intro.md §5`, `systems/display-driver.md §8`) | Four `320x49` bands that exist only in the EGA driver's runtime back-buffer and cannot be read from an external art file; clean engines should author replacements. | `ULTIMA` slots 1..=4 (288x49, 288x49, 288x49, 288x50) are the four flaming "Warriors of Destiny" bands. The upper 49 rows of each are blitted at `(16, 65)`; columns 0..=15 and 304..=319 of the published `(0, 65)` 320x49 rectangle are cleared to index 0, so the destination is still overwritten opaquely. The mod-4 advance and cadence are unchanged. Frame 0 = slot 1 is assumed (the captures pin slots 2 and 3 only). |
| Acknowledgements art (`#72`, `systems/intro.md §11`) | "Loads its own graphics resource from the end-screen asset family". | `ENDSC.16` is a single blank 260x168 parchment; `STARTSC.16` is the credits artwork. Pressing `A` keeps the `ULTIMA` logo on rows 0..=60 and replaces rows 63..=199 with the STARTSC 16 + 288 + 16 by 137 composition, bottom-aligned. No text is drawn over it - the credits are part of the artwork - and any key restores the menu. `#72` is closed: the entry and exit are the published rise/part/close/sink phase sequence, not the withdrawn horizontal slab wipes. |
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
- Read contracts from `cleak/u5-spec` on GitHub - the issues, and document
  text through `gh api -H "Accept: application/vnd.github.raw"
  repos/cleak/u5-spec/contents/<path>`. The local checkout at
  `C:\Projects\Rust\u5-clean\u5-spec` is read-only from this workspace and is
  stale at `9a898d1`, many commits and several retractions behind spec head -
  the reconciliation table would look wrong if checked against it.
- The shipped palette is not stock EGA: index 6 is `(170, 170, 0)` dark yellow,
  not `(170, 85, 0)` brown, and it is the only index that differs. `STORY1`
  slide 0 is 25% index 6 and the `STARTSC` acknowledgements parchment 6%, so
  both changed hue when it was corrected. Nothing reprograms the palette after
  mode setup.
- Intro evidence in `--visual-frame-suite`: `intro-menu`, `intro-finished-menu`,
  `intro-story-00`..`intro-story-20`, `intro-return-to-view`,
  `intro-chargen-name-prompt`, `intro-chargen-gender-prompt`,
  `intro-chargen-gender-echo`, `intro-acknowledgements-risen`/`-parting`/
  `-credits`/`-closing`, `intro-u4-transfer-found`, `intro-u4-transfer-panels`.
  193 PNGs total on 2026-08-23 at `9e437d5`.
- The suite used to render no menu window at all while the live path was
  correct, because it built its intro state through a parallel path. It now
  drives the real render path, so a defect of that shape cannot hide there
  again.

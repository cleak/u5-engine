# Intro Graphics Gaps

Last refreshed: 2026-08-22 from `cleak/u5-spec` open issues and the current
`u5-engine` tree.

This file tracks the intro graphics contracts that still block an authentic
minimum intro implementation under the strict no-fallback rule. If a behavior is
listed here, `u5-engine` should fail loudly rather than render invented text,
generated animation, cropped buffers, or diagnostic substitutes.

## Awaiting Spec Response

| Issue | Blocking contract | Current engine behavior |
|---:|---|---|
| [#54](https://github.com/cleak/u5-spec/issues/54) | Return-to-View preview pixel origin, preview-cell size/source, actor pixel mapping, wipe alignment, and full-clear vs dirty-rect behavior. | Bevy validates command playback, then rejects the invalid full-tile `64x304` preview instead of cropping/scaling. Terminal `--intro` rejects Return-to-View diagnostics when `MISCMAPS.DAT` exists. |
| [#68](https://github.com/cleak/u5-spec/issues/68) | Phase-specific intro animation cadence and host catch-up behavior for title flourish, signature path, STARTSC reveal, story reveal, and Return-to-View ticks. | Bevy rejects wall-clock-driven intro playback instead of using the temporary shared-pump assumption. |
| [#69](https://github.com/cleak/u5-spec/issues/69) | Exact inline doorway text and layout for intro story step 6. | Bevy rejects rendering, advancing, or cancelling past step 6; terminal story diagnostics are rejected when `STORY.DAT` exists. |
| [#70](https://github.com/cleak/u5-spec/issues/70) | Resident proportional width table, spacing/wrap/centering rules, and Return-to-View caption text layout. | Bevy rejects proportional intro text and caption fallback rendering. |
| [#71](https://github.com/cleak/u5-spec/issues/71) | Initial title/rune screen visual contract and input behavior before `TITLE.BIT` flourish. | Bevy rejects initial title render, ticks, and key input instead of skipping to the menu. |
| [#72](https://github.com/cleak/u5-spec/issues/72) | Entry/exit wipe cadence for the acknowledgements parchment. | Bevy draws the observed artwork (see below) and blits it immediately instead of inventing a sweep. Terminal `--intro` still rejects placeholder acknowledgements, having no way to draw the art. |
| [#73](https://github.com/cleak/u5-spec/issues/73) | Ultima IV transfer roster/status preview, prompt window, confirmation prompts, redraw timing, and page behavior. | Bevy and terminal `--intro` reject transfer preview fallbacks before save commit. |

## Answered Or Non-Blocking For This Pass

| Issue | Status |
|---:|---|
| [#52](https://github.com/cleak/u5-spec/issues/52) / [#65](https://github.com/cleak/u5-spec/issues/65) / [#63](https://github.com/cleak/u5-spec/issues/63) | Resolved by clean runtime observation of the shipped assets, pending the spec correction requested as [#78](https://github.com/cleak/u5-spec/issues/78). See "Resolved By Runtime Observation" below. |
| [#64](https://github.com/cleak/u5-spec/issues/64) | CREATE/chargen presentation was published and implemented; no current response is needed for the chargen graphics contract. |
| [#66](https://github.com/cleak/u5-spec/issues/66) / [#67](https://github.com/cleak/u5-spec/issues/67) | Title bitmap layering and flourish slot presentation were answered and implemented; remaining title blockers are now the initial screen and cadence above; the title-tick frame content is resolved by runtime observation. |

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

- The current strict path intentionally makes the intro less runnable until the
  contracts above are published; this prevents broken graphics from being hidden
  behind text or generated-art substitutes.
- When one of these issues is answered, remove only the corresponding panic and
  add focused tests that prove the published contract is being followed.

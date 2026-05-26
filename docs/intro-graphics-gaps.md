# Intro Graphics Gaps

Last refreshed: 2026-05-26 from `cleak/u5-spec` open issues and the current
`u5-engine` tree.

This file tracks the intro graphics contracts that still block an authentic
minimum intro implementation under the strict no-fallback rule. If a behavior is
listed here, `u5-engine` should fail loudly rather than render invented text,
generated animation, cropped buffers, or diagnostic substitutes.

## Awaiting Spec Response

| Issue | Blocking contract | Current engine behavior |
|---:|---|---|
| [#52](https://github.com/cleak/u5-spec/issues/52) / [#65](https://github.com/cleak/u5-spec/issues/65) | Four authored `320x49` title-tick frames, or a pixel-exact procedural contract, plus the authoritative sparse-vs-opaque overwrite rule. | Bevy rejects title-tick drawing instead of using generated flame art. |
| [#54](https://github.com/cleak/u5-spec/issues/54) | Return-to-View preview pixel origin, preview-cell size/source, actor pixel mapping, wipe alignment, and full-clear vs dirty-rect behavior. | Bevy validates command playback, then rejects the invalid full-tile `64x304` preview instead of cropping/scaling. Terminal `--intro` rejects Return-to-View diagnostics when `MISCMAPS.DAT` exists. |
| [#63](https://github.com/cleak/u5-spec/issues/63) | Lower intro menu/text-window frame bounds, source primitive, glyphs/colors, clear rules, and draw order. | Bevy rejects the six-option menu render instead of showing a plain black text band. |
| [#68](https://github.com/cleak/u5-spec/issues/68) | Phase-specific intro animation cadence and host catch-up behavior for title flourish, signature path, STARTSC reveal, story reveal, and Return-to-View ticks. | Bevy rejects wall-clock-driven intro playback instead of using the temporary shared-pump assumption. |
| [#69](https://github.com/cleak/u5-spec/issues/69) | Exact inline doorway text and layout for intro story step 6. | Bevy rejects rendering, advancing, or cancelling past step 6; terminal story diagnostics are rejected when `STORY.DAT` exists. |
| [#70](https://github.com/cleak/u5-spec/issues/70) | Resident proportional width table, spacing/wrap/centering rules, and Return-to-View caption text layout. | Bevy rejects proportional intro text and caption fallback rendering. |
| [#71](https://github.com/cleak/u5-spec/issues/71) | Initial title/rune screen visual contract and input behavior before `TITLE.BIT` flourish. | Bevy rejects initial title render, ticks, and key input instead of skipping to the menu. |
| [#72](https://github.com/cleak/u5-spec/issues/72) | Acknowledgement/credits text, layout, page/input behavior, and backing-surface handling. | Bevy rejects placeholder acknowledgements. |
| [#73](https://github.com/cleak/u5-spec/issues/73) | Ultima IV transfer roster/status preview, prompt window, confirmation prompts, redraw timing, and page behavior. | Bevy and terminal `--intro` reject transfer preview fallbacks before save commit. |

## Answered Or Non-Blocking For This Pass

| Issue | Status |
|---:|---|
| [#64](https://github.com/cleak/u5-spec/issues/64) | CREATE/chargen presentation was published and implemented; no current response is needed for the chargen graphics contract. |
| [#66](https://github.com/cleak/u5-spec/issues/66) / [#67](https://github.com/cleak/u5-spec/issues/67) | Title bitmap layering and flourish slot presentation were answered and implemented; remaining title blockers are now the initial screen, cadence, and title-tick frame content above. |

## Verification Notes

- The current strict path intentionally makes the intro less runnable until the
  contracts above are published; this prevents broken graphics from being hidden
  behind text or generated-art substitutes.
- When one of these issues is answered, remove only the corresponding panic and
  add focused tests that prove the published contract is being followed.

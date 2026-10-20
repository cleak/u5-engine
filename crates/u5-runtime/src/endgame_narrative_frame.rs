//! `endgame.md §8.3`: the composition of one final narrative window, shared
//! by every frontend that draws a whole screen.
//!
//! The six windows are a hard cut onto the **full-screen** window: per
//! window the hidden page is cleared, the title strips (if any) and then the
//! opaque panel are drawn, the window's layout descriptor is installed and
//! its `END.DAT` record is laid out with the proportional renderer; the page
//! is then published in one copy. Nothing of the gameplay layout - chrome,
//! stats panel, the sixteen-column message window, the viewport - survives
//! onto it.
//!
//! The visual shell has composed the window this way since playtest F58; the
//! headless `--save-screen` composer and the plain-text frames did not, and
//! drew the prose into the message window over the dungeon viewport (F58
//! remainder). Both now take the page from here.

use std::io;
use std::path::Path;

use crate::endgame_cinematic::EndgameCinematicStep;
use crate::{
    EGA_PALETTE_RGB, FULL_SCREEN_TEXT_WINDOW_INDEX, PCS_GLYPH_HEIGHT, PROPORTIONAL_WIDTH_TABLE,
    PlayState, TEXT_CTRL_CLEAR_WINDOW, TEXT_SCREEN_COLUMNS, TEXT_SCREEN_ROWS,
    TEXT_WINDOW_RENDER_HEIGHT, TEXT_WINDOW_RENDER_WIDTH, TextWindowSystem, TileGraphicsDepth,
    load_graphic_image_directory, load_proportional_font,
};

/// `endgame.md §8.2`: "every set bit of a proportional glyph is stamped as
/// palette index 15 (white), while unset bits leave the panel or page
/// background unchanged."
pub const ENDGAME_NARRATIVE_INK_INDEX: u8 = 15;

/// Zero-based index of the `endgame.md §8` narrative window on screen, or
/// `None` when the current frame is not one of the six.
pub fn endgame_narrative_window_on_screen(state: &PlayState) -> Option<usize> {
    match state.endgame.as_ref()?.cinematic.step {
        EndgameCinematicStep::NarrativeWindow(index) => Some(usize::from(index)),
        _ => None,
    }
}

/// Compose narrative window `window_index` (zero-based) as a full 320x200
/// page of EGA palette indices, in `§8.3`'s order.
pub fn compose_endgame_narrative_window_indices(
    state: &PlayState,
    game_dir: &Path,
    window_index: usize,
) -> io::Result<Vec<u8>> {
    let invalid = |message: String| io::Error::new(io::ErrorKind::InvalidData, message);
    let panel = crate::story_layout::endgame_narrative_panel(window_index).ok_or_else(|| {
        invalid(format!(
            "endgame narrative window {window_index} is outside the published six"
        ))
    })?;
    let descriptor = crate::story_layout::endgame_narrative_paragraph_box(window_index)
        .ok_or_else(|| {
            invalid(format!(
                "endgame narrative window {window_index} has no published paragraph rectangle"
            ))
        })?;
    let width = TEXT_WINDOW_RENDER_WIDTH;
    let height = TEXT_WINDOW_RENDER_HEIGHT;
    // Step 3: the clear control over the full-screen window blanks the page.
    let mut page = vec![0u8; width * height];
    let mut draw_record = |stem: &str,
                           slot: u8,
                           x: u16,
                           y: u16,
                           expected: Option<(usize, usize)>|
     -> io::Result<()> {
        let directory = load_graphic_image_directory(game_dir, stem, TileGraphicsDepth::Ega16)?;
        let image = directory
            .images
            .get(usize::from(slot))
            .and_then(Option::as_ref)
            .ok_or_else(|| invalid(format!("endgame archive {stem} is missing slot {slot}")))?;
        if let Some((w, h)) = expected
            && (image.width, image.height) != (w, h)
        {
            return Err(invalid(format!(
                "endgame panel {stem} slot {slot} is {}x{}, but `endgame.md §8.1` publishes {w}x{h}",
                image.width, image.height
            )));
        }
        let (x, y) = (usize::from(x), usize::from(y));
        if x + image.width > width || y + image.height > height {
            return Err(invalid(format!(
                "endgame archive {stem} slot {slot} at ({x}, {y}) exceeds the page"
            )));
        }
        // Opaque: no border, no shadow, no frame (`§8.1`).
        for row in 0..image.height {
            let src = &image.pixels[row * image.width..(row + 1) * image.width];
            let dst = (y + row) * width + x;
            for (cell, pixel) in page[dst..dst + image.width].iter_mut().zip(src) {
                *cell = pixel & 0x0f;
            }
        }
        Ok(())
    };
    // Step 4, then step 5 over it (R272).
    for strip in crate::story_layout::endgame_narrative_title_strips(window_index) {
        draw_record(
            crate::story_layout::ENDGAME_TITLE_STRIP_ARCHIVE,
            strip.slot,
            strip.x,
            strip.y,
            None,
        )?;
    }
    draw_record(
        panel.archive,
        panel.slot,
        panel.top_left_x,
        panel.top_left_y,
        Some((usize::from(panel.width), usize::from(panel.height))),
    )?;
    // Steps 6 and 7: the record with its `{` indent and `_` soft hyphens
    // intact for the proportional layout (`formats/end-dat.md §3`).
    let prose = state
        .endgame
        .as_ref()
        .and_then(|endgame| endgame.current_narrative_layout_text())
        .unwrap_or_else(|| state.message.clone());
    let font = load_proportional_font(game_dir)?;
    let placed = crate::story_layout::layout_proportional_paragraph_glyphs(
        &PROPORTIONAL_WIDTH_TABLE,
        &descriptor,
        prose.as_bytes(),
        crate::story_layout::PROPORTIONAL_DRAW_CLIP_Y,
    )?;
    for glyph in placed {
        let ink_width = PROPORTIONAL_WIDTH_TABLE.width_for_byte(glyph.code)?;
        let bitmap = font
            .glyph_for_code(glyph.code)
            .ok_or_else(|| invalid(format!("PROPORT.PCS has no glyph for code {}", glyph.code)))?;
        let (x, y) = (usize::from(glyph.x), usize::from(glyph.y));
        let rows = bitmap.bitmap.height.min(PCS_GLYPH_HEIGHT);
        for row in 0..rows.min(height.saturating_sub(y)) {
            for col in 0..ink_width.min(width.saturating_sub(x)) {
                if bitmap.bitmap.pixel(col, row) == Some(1) {
                    page[(y + row) * width + x + col] = ENDGAME_NARRATIVE_INK_INDEX;
                }
            }
        }
    }
    // `§7.1`'s fade, if a rectangle of it is still standing.
    if let Some((x0, y0, x1, y1)) = state
        .endgame
        .as_ref()
        .and_then(|endgame| endgame.cinematic.fade_to_black_rect)
    {
        let x1 = usize::from(x1).min(width - 1);
        let y1 = usize::from(y1).min(height - 1);
        for y in usize::from(y0)..=y1 {
            for x in usize::from(x0)..=x1 {
                page[y * width + x] = 0;
            }
        }
    }
    Ok(page)
}

/// [`compose_endgame_narrative_window_indices`] as opaque RGBA.
pub fn compose_endgame_narrative_window_rgba(
    state: &PlayState,
    game_dir: &Path,
    window_index: usize,
) -> io::Result<Vec<u8>> {
    let page = compose_endgame_narrative_window_indices(state, game_dir, window_index)?;
    let mut rgba = Vec::with_capacity(page.len() * 4);
    for index in page {
        let rgb = EGA_PALETTE_RGB[usize::from(index & 0x0f)];
        rgba.extend_from_slice(&[rgb[0], rgb[1], rgb[2], 0xff]);
    }
    Ok(rgba)
}

/// The plain-text frame's stand-in for a narrative window: the full-screen
/// window cleared and the window's marker-free prose printed into it from
/// the top-left. A fixed-cell surface cannot reproduce the proportional
/// layout, but it can keep `§8.3`'s page model - nothing of the message
/// window, the stats panel or the viewport is on it.
pub fn endgame_narrative_full_screen_text_rows(state: &PlayState) -> Option<Vec<String>> {
    endgame_narrative_window_on_screen(state)?;
    let mut system = TextWindowSystem::new();
    system.set_window_rect(
        FULL_SCREEN_TEXT_WINDOW_INDEX,
        0,
        0,
        TEXT_SCREEN_COLUMNS - 1,
        TEXT_SCREEN_ROWS - 1,
    );
    system.set_active_window(FULL_SCREEN_TEXT_WINDOW_INDEX);
    system.emit_byte(TEXT_CTRL_CLEAR_WINDOW);
    system.set_active_cursor(0, 0);
    system.print_wrapped_string(&state.message);
    Some(system.screen_rows(b' '))
}

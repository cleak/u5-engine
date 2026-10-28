//! `endgame.md §8.4`/`§9`: the certificate page, shared by every frontend
//! that draws a whole screen.
//!
//! After the sixth narrative window and one more key, the presentation
//! clears the page, draws the end-screen archive's only record - the blank
//! 260-by-168 parchment - at `(40, 0)`, publishes it, and prints the
//! certificate onto it with the fixed-cell text path: centred, inverse for
//! the body and the runic closing title, normal video for the elapsed-time
//! report below the parchment. `§9.5`: nothing follows, so this page is the
//! terminal screen.
//!
//! The visual shell has drawn it since `cleak/u5-spec#82`; the headless
//! `--save-screen` composer and the plain-text frames fell back to the
//! gameplay layout over the dungeon the party left (playtest P15-06). Both
//! now take the page from here.

use std::io;
use std::path::Path;

use crate::endgame_cinematic::EndgameCinematicStep;
use crate::{
    CH_CELL_SIDE, EGA_PALETTE_RGB, ENDGAME_CERTIFICATE_PARCHMENT_X,
    ENDGAME_CERTIFICATE_PARCHMENT_Y, EndgameCertificateLine, FixedCellFont, PlayState,
    TEXT_SCREEN_COLUMNS, TEXT_SCREEN_ROWS, TEXT_WINDOW_RENDER_HEIGHT, TEXT_WINDOW_RENDER_WIDTH,
    TileGraphicsDepth, endgame_certificate_lines, load_graphic_image_directory,
    runic_line_encoding,
};

/// `endgame.md §8.4`: the end-screen archive holding the parchment.
pub const ENDGAME_CERTIFICATE_ARCHIVE: &str = "ENDSC";
/// `endgame.md §8.4`: the parchment is the archive's only record,
/// 260 by 168 pixels.
pub const ENDGAME_CERTIFICATE_PARCHMENT_SIZE: (usize, usize) = (260, 168);

/// Whether the certificate page owns the screen: the certificate beat
/// onward, including `§9.5`'s terminal hold.
pub fn endgame_certificate_on_screen(state: &PlayState) -> bool {
    state.endgame.as_ref().is_some_and(|endgame| {
        matches!(
            endgame.cinematic.step,
            EndgameCinematicStep::Certificate
                | EndgameCinematicStep::FinalReport
                | EndgameCinematicStep::Finished
        ) && endgame.certificate.is_some()
            && endgame.final_report.is_some()
    })
}

/// The printed certificate rows, or `None` when the page is not up.
pub fn endgame_certificate_page_lines(state: &PlayState) -> Option<Vec<EndgameCertificateLine>> {
    if !endgame_certificate_on_screen(state) {
        return None;
    }
    let endgame = state.endgame.as_ref()?;
    Some(endgame_certificate_lines(
        endgame.certificate.as_ref()?,
        endgame.final_report?,
    ))
}

/// Compose the certificate page as 320x200 EGA palette indices: the
/// cleared page, the parchment at `(40, 0)`, then every certificate line
/// as whole fixed-cell glyphs (inverse swaps ink and paper).
pub fn compose_endgame_certificate_indices(
    state: &PlayState,
    game_dir: &Path,
    ibm: &FixedCellFont,
    runes: &FixedCellFont,
) -> io::Result<Vec<u8>> {
    let invalid = |message: String| io::Error::new(io::ErrorKind::InvalidData, message);
    let lines = endgame_certificate_page_lines(state)
        .ok_or_else(|| invalid("the endgame certificate is not on screen".to_string()))?;
    let width = TEXT_WINDOW_RENDER_WIDTH;
    let height = TEXT_WINDOW_RENDER_HEIGHT;
    let mut page = vec![0u8; width * height];

    let directory = load_graphic_image_directory(
        game_dir,
        ENDGAME_CERTIFICATE_ARCHIVE,
        TileGraphicsDepth::Ega16,
    )?;
    let image = directory
        .images
        .first()
        .and_then(Option::as_ref)
        .ok_or_else(|| {
            invalid(format!(
                "{ENDGAME_CERTIFICATE_ARCHIVE} has no parchment record"
            ))
        })?;
    if (image.width, image.height) != ENDGAME_CERTIFICATE_PARCHMENT_SIZE {
        return Err(invalid(format!(
            "certificate parchment is {}x{}, but `endgame.md §8.4` publishes 260x168",
            image.width, image.height
        )));
    }
    let (x0, y0) = (
        ENDGAME_CERTIFICATE_PARCHMENT_X,
        ENDGAME_CERTIFICATE_PARCHMENT_Y,
    );
    if x0 + image.width > width || y0 + image.height > height {
        return Err(invalid(
            "certificate parchment exceeds the page".to_string(),
        ));
    }
    for row in 0..image.height {
        let src = &image.pixels[row * image.width..(row + 1) * image.width];
        let dst = (y0 + row) * width + x0;
        for (cell, pixel) in page[dst..dst + image.width].iter_mut().zip(src) {
            *cell = pixel & 0x0f;
        }
    }

    for line in lines {
        let (ink, paper) = if line.inverse {
            (0x00, 0x0f)
        } else {
            (0x0f, 0x00)
        };
        let font = if line.runic { runes } else { ibm };
        let text = if line.runic {
            runic_line_encoding(&line.text)
        } else {
            line.text.clone()
        };
        let column = usize::from(line.column());
        let row = usize::from(line.row);
        for (offset, byte) in text.bytes().enumerate() {
            let cell_x = column + offset;
            if cell_x >= usize::from(TEXT_SCREEN_COLUMNS) || row >= usize::from(TEXT_SCREEN_ROWS) {
                break;
            }
            for glyph_y in 0..CH_CELL_SIDE {
                let bits = font.glyph_row(byte & 0x7f, glyph_y).ok_or_else(|| {
                    invalid(format!(
                        "fixed font glyph {} is missing row {glyph_y}",
                        byte & 0x7f
                    ))
                })?;
                for glyph_x in 0..CH_CELL_SIDE {
                    let colour = if bits & (1 << (7 - glyph_x)) != 0 {
                        ink
                    } else {
                        paper
                    };
                    let (x, y) = (
                        cell_x * CH_CELL_SIDE + glyph_x,
                        row * CH_CELL_SIDE + glyph_y,
                    );
                    page[y * width + x] = colour;
                }
            }
        }
    }
    Ok(page)
}

/// [`compose_endgame_certificate_indices`] as opaque RGBA.
pub fn compose_endgame_certificate_rgba(
    state: &PlayState,
    game_dir: &Path,
    ibm: &FixedCellFont,
    runes: &FixedCellFont,
) -> io::Result<Vec<u8>> {
    let page = compose_endgame_certificate_indices(state, game_dir, ibm, runes)?;
    let mut rgba = Vec::with_capacity(page.len() * 4);
    for index in page {
        let rgb = EGA_PALETTE_RGB[usize::from(index & 0x0f)];
        rgba.extend_from_slice(&[rgb[0], rgb[1], rgb[2], 0xff]);
    }
    Ok(rgba)
}

/// The plain-text frame's stand-in for the certificate page: the
/// full-screen window with every certificate line at its row and centred
/// column. The closing title is shown in its decoded Latin form, placed by
/// the encoded width the page uses (`§9.3`).
pub fn endgame_certificate_full_screen_text_rows(state: &PlayState) -> Option<Vec<String>> {
    let lines = endgame_certificate_page_lines(state)?;
    let columns = usize::from(TEXT_SCREEN_COLUMNS);
    let mut rows = vec![vec![' '; columns]; usize::from(TEXT_SCREEN_ROWS)];
    for line in lines {
        let Some(row) = rows.get_mut(usize::from(line.row)) else {
            continue;
        };
        for (offset, ch) in line.text.chars().enumerate() {
            if let Some(cell) = row.get_mut(usize::from(line.column()) + offset) {
                *cell = ch;
            }
        }
    }
    Some(
        rows.into_iter()
            .map(|row| row.into_iter().collect())
            .collect(),
    )
}

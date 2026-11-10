//! The grow-left compositor, steps 5 and 6 (Phase B design 2.2.3, 2.2.6,
//! 2.3.7, 2.3.8; build plan M1-10): the essentials ribbon in the top band,
//! with its health colours, food, gold and the date, and the sky and wind
//! tabs cropped from the faithful frame and placed at one shared centre.
//!
//! **What the ribbon may show** (2.2.9). Every character it draws is one
//! the faithful stats panel is showing: the names, HP numbers and status
//! bytes are read from the panel body on screen
//! (`PlayState::displayed_stats_panel`, the rows the faithful frame paints,
//! a staged presentation's held body included), and food, gold and the
//! date from [`stats_panel_counter_text`] and [`stats_panel_date_text`]. Only
//! members of the current party are shown (at most the active-party cap
//! of six). The ribbon adds three marks of its own and no words: the
//! colon after a name, a `+` for an HP wider than three cells, and the HP
//! colour, which restates the number (red) or the `P` status letter
//! (green). Maximum HP, which decides red, is read from `party` and never
//! drawn.
//!
//! **When** (D2.11). The step takes a [`BandSnapshot`] only on a frame
//! whose faithful bytes changed, so nothing appears before the original
//! screen could show it.
//!
//! Glyphs are drawn from the runtime-loaded IBM.CH straight onto the blue
//! band (foreground pixels only), so a status byte that the shared emitter
//! would treat as a control code is only ever a glyph here.

use u5_runtime::stats_panel::{STATS_PANEL_NAME_CELLS, StatsPanelSnapshot};
use u5_runtime::{
    CH_CELL_SIDE, ChromeGapExtents, FixedCellFont, GapPixelRect, PlayState,
    gameplay_chrome_content, gameplay_chrome_gap_extents, stats_panel_counter_text,
    stats_panel_date_text,
};

use super::compose::RULE_WHITE;
use super::{COLUMN_CLEAR_X, Canvas, SIDE_BORDER};
use crate::EGA_PALETTE_RGB;

/// The active-party cap: the ribbon's two rows by three columns.
pub const RIBBON_MEMBERS: usize = 6;
/// Light red (EGA 12): HP at most a quarter of maximum (D2.17).
pub const HP_LOW: u8 = 12;
/// Light green (EGA 10): the status letter is `P` (poisoned wins).
pub const HP_POISONED: u8 = 10;
/// The status byte that turns HP green.
pub const STATUS_POISONED: u8 = b'P';
/// HP is right-aligned in three cells; a wider HP shows this in the last.
pub const HP_CELLS: usize = 3;
pub const HP_OVERFLOW: u8 = b'+';
/// The narrowest canvas that shows full names (the 21:9 layout, 688).
pub const FULL_NAMES_FROM: u32 = 688;
/// Full names: the name and colon padded to nine cells (names are at most
/// eight characters).
pub const FULL_NAME_FIELD: usize = 9;
/// Initials: the initial and colon, two cells.
pub const INITIAL_FIELD: usize = 2;
/// The ribbon's left margin and the counters' right margin: 8 pixels
/// inside the rule.
pub const BAND_MARGIN: u32 = 8;
/// The gap the 21:9 ribbon keeps on each side of the sky tab's strip
/// (x 232..279 and 408..455 at 688).
pub const FULL_NAME_TAB_GAP: u32 = 48;
/// The 128-pixel strip the mockups centre the sky tab in (frame x 32..159).
pub const TAB_STRIP: u32 = 128;

const CELL: u32 = CH_CELL_SIDE as u32;

/// One member as the faithful panel shows them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RibbonMember {
    /// The name field of the panel row, trailing padding removed.
    pub name: String,
    /// The HP field of the panel row, padding removed.
    pub hp: String,
    /// The raw status byte the panel draws (`None`: a blank cell).
    pub status: Option<u8>,
    /// Maximum HP, for the colour only; never drawn.
    pub max_hp: u16,
}

impl RibbonMember {
    /// The HP number, when the field is a plain number.
    pub fn hp_value(&self) -> Option<u32> {
        self.hp.parse().ok()
    }
}

/// The ribbon's text (2.3.8).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RibbonSnapshot {
    /// Roster order, at most [`RIBBON_MEMBERS`].
    pub members: Vec<RibbonMember>,
    /// The counters row with its padding runs closed to one space.
    pub counters: String,
    /// The date row, likewise.
    pub date: String,
}

/// Everything the top and bottom bands draw from the game, taken together
/// on a frame whose faithful bytes changed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BandSnapshot {
    pub ribbon: RibbonSnapshot,
    /// The tab extents the faithful chrome pass painted.
    pub gaps: ChromeGapExtents,
}

/// Collapse every run of spaces to one and trim the ends
/// (`stats-panel.md §6`, `§7`: the rows' padding runs).
pub fn close_runs(text: &str) -> String {
    text.split(' ')
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn member_of(panel: &StatsPanelSnapshot, state: &PlayState, index: usize) -> Option<RibbonMember> {
    let row: Vec<char> = panel.rows.get(index)?.chars().collect();
    let member = state.party.get(index)?;
    // The row: the name field, the marker cell, HP right-justified (four
    // cells, or wider for an edited HP), then the status cell.
    let name: String = row.iter().take(STATS_PANEL_NAME_CELLS).collect();
    let hp: String = row
        .iter()
        .skip(STATS_PANEL_NAME_CELLS + 1)
        .take(row.len().saturating_sub(STATS_PANEL_NAME_CELLS + 2))
        .collect();
    Some(RibbonMember {
        name: name.trim_end().to_string(),
        hp: hp.trim().to_string(),
        status: panel.statuses.get(index).copied().flatten(),
        max_hp: member.max_hp,
    })
}

/// The ribbon for `state`, read from the panel the faithful frame paints.
pub fn ribbon_snapshot(state: &PlayState) -> RibbonSnapshot {
    let panel = state.displayed_stats_panel();
    let members = (0..RIBBON_MEMBERS.min(state.party.len()))
        .filter_map(|index| member_of(panel, state, index))
        .collect();
    RibbonSnapshot {
        members,
        counters: close_runs(stats_panel_counter_text(state)),
        date: close_runs(stats_panel_date_text(state)),
    }
}

/// The ribbon and the tab extents for `state`.
pub fn band_snapshot(state: &PlayState) -> BandSnapshot {
    BandSnapshot {
        ribbon: ribbon_snapshot(state),
        gaps: gameplay_chrome_gap_extents(&gameplay_chrome_content(state)),
    }
}

/// The HP colour (`display.ribbon_hp_colours`, D2.17): green when the
/// status is `P` (poisoned wins), red when HP is at most a quarter of
/// maximum, white otherwise or with the toggle off. Nothing blinks.
pub fn ribbon_hp_colour(member: &RibbonMember, enabled: bool) -> u8 {
    if !enabled {
        return RULE_WHITE;
    }
    if member.status == Some(STATUS_POISONED) {
        return HP_POISONED;
    }
    match member.hp_value() {
        Some(hp) if u64::from(hp) * 4 <= u64::from(member.max_hp) => HP_LOW,
        _ => RULE_WHITE,
    }
}

/// The ribbon's geometry on one canvas.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RibbonLayout {
    /// Full names (21:9 and wider) or initials.
    pub full_names: bool,
    /// The three columns' left edges.
    pub columns: [u32; 3],
}

impl RibbonLayout {
    /// The name-and-colon field in cells.
    pub fn name_field(&self) -> usize {
        if self.full_names {
            FULL_NAME_FIELD
        } else {
            INITIAL_FIELD
        }
    }

    /// One member's width in cells: the name field, HP and status.
    pub fn member_cells(&self) -> usize {
        self.name_field() + HP_CELLS + 1
    }

    /// Where member `index` goes: down then across, column `index / 2`,
    /// row `index % 2`.
    pub fn place(&self, index: usize) -> (u32, u32) {
        (self.columns[index / 2], (index % 2) as u32 * CELL)
    }
}

/// The ribbon layout for `canvas` (2.2.3). Initials: columns at x 16, 72
/// and 128 (a seven-cell pitch), in both states. Full names: x 16 and 128,
/// and the third column 48 pixels right of the sky tab's strip when the
/// column is collapsed (x 456 at 688), or at x 240 when it is open.
pub fn ribbon_layout(canvas: &Canvas, open: bool) -> RibbonLayout {
    let left = SIDE_BORDER + BAND_MARGIN;
    if canvas.w < FULL_NAMES_FROM {
        let pitch = (INITIAL_FIELD as u32 + HP_CELLS as u32 + 2) * CELL;
        return RibbonLayout {
            full_names: false,
            columns: [left, left + pitch, left + 2 * pitch],
        };
    }
    let pitch = (FULL_NAME_FIELD as u32 + HP_CELLS as u32 + 2) * CELL;
    let third = if open {
        left + 2 * pitch
    } else {
        canvas.w / 2 + TAB_STRIP / 2 + FULL_NAME_TAB_GAP
    };
    RibbonLayout {
        full_names: true,
        columns: [left, left + pitch, third],
    }
}

/// One member's ribbon cells: the name field, HP right-aligned in three
/// cells (a `+` in the last when wider), then the status byte.
pub fn member_cells(member: &RibbonMember, layout: &RibbonLayout) -> Vec<(u8, CellPart)> {
    let mut cells = Vec::with_capacity(layout.member_cells());
    let name: Vec<u8> = member.name.chars().map(glyph_code).collect();
    let shown = if layout.full_names {
        &name[..name.len().min(FULL_NAME_FIELD - 1)]
    } else {
        &name[..name.len().min(1)]
    };
    for byte in shown {
        cells.push((*byte, CellPart::Name));
    }
    cells.push((b':', CellPart::Name));
    while cells.len() < layout.name_field() {
        cells.push((b' ', CellPart::Name));
    }
    let hp: Vec<u8> = member.hp.chars().map(glyph_code).collect();
    let hp: Vec<u8> = if hp.len() > HP_CELLS {
        let mut cut = hp[..HP_CELLS - 1].to_vec();
        cut.push(HP_OVERFLOW);
        cut
    } else {
        let mut padded = vec![b' '; HP_CELLS - hp.len()];
        padded.extend(hp);
        padded
    };
    cells.extend(hp.into_iter().map(|byte| (byte, CellPart::Hp)));
    cells.push((member.status.unwrap_or(b' '), CellPart::Status));
    cells
}

/// Which field a ribbon cell belongs to (for its colour).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CellPart {
    Name,
    Hp,
    Status,
}

/// The font cell for a character of panel text: the low seven bits, as the
/// panel's own glyph blit takes them.
fn glyph_code(ch: char) -> u8 {
    (u32::from(ch) & 0x7f) as u8
}

/// Draw one glyph's foreground pixels at `(x, y)` in `colour`, clipped to
/// the canvas.
fn draw_glyph(
    rgba: &mut [u8],
    canvas: &Canvas,
    font: &FixedCellFont,
    code: u8,
    (x, y): (u32, u32),
    colour: u8,
) {
    let [r, g, b] = EGA_PALETTE_RGB[usize::from(colour & 0x0f)];
    let w = canvas.w as usize;
    for row in 0..CH_CELL_SIDE {
        let Some(bits) = font.glyph_row(code & 0x7f, row) else {
            continue;
        };
        for col in 0..CH_CELL_SIDE {
            if bits & (0x80 >> col) == 0 {
                continue;
            }
            let (px, py) = (x as usize + col, y as usize + row);
            if px >= w || py >= canvas.h as usize {
                continue;
            }
            let at = (py * w + px) * 4;
            rgba[at..at + 3].copy_from_slice(&[r, g, b]);
        }
    }
}

fn draw_text(
    rgba: &mut [u8],
    canvas: &Canvas,
    font: &FixedCellFont,
    text: &str,
    (x, y): (u32, u32),
    colour: u8,
) {
    for (i, ch) in text.chars().enumerate() {
        draw_glyph(
            rgba,
            canvas,
            font,
            glyph_code(ch),
            (x + i as u32 * CELL, y),
            colour,
        );
    }
}

/// The right end (exclusive) of the counters and the date: 8 pixels inside
/// the right rule.
pub fn counters_right(canvas: &Canvas) -> u32 {
    canvas.w - SIDE_BORDER - BAND_MARGIN
}

/// The left edge of the right-aligned food/gold (row y 0) and date (y 8)
/// text, whichever starts further left.
pub fn counters_left(canvas: &Canvas, ribbon: &RibbonSnapshot) -> u32 {
    let widest = ribbon
        .counters
        .chars()
        .count()
        .max(ribbon.date.chars().count()) as u32;
    counters_right(canvas).saturating_sub(widest * CELL)
}

/// Step 5: the ribbon, food/gold and the date on the top band.
pub fn paint_ribbon(
    rgba: &mut [u8],
    canvas: &Canvas,
    ribbon: &RibbonSnapshot,
    font: &FixedCellFont,
    hp_colours: bool,
    open: bool,
) {
    let layout = ribbon_layout(canvas, open);
    for (index, member) in ribbon.members.iter().take(RIBBON_MEMBERS).enumerate() {
        let (x, y) = layout.place(index);
        let hp_colour = ribbon_hp_colour(member, hp_colours);
        for (i, (code, part)) in member_cells(member, &layout).into_iter().enumerate() {
            let colour = if part == CellPart::Hp {
                hp_colour
            } else {
                RULE_WHITE
            };
            draw_glyph(rgba, canvas, font, code, (x + i as u32 * CELL, y), colour);
        }
    }
    let right = counters_right(canvas);
    for (text, y) in [(&ribbon.counters, 0), (&ribbon.date, CELL)] {
        let width = text.chars().count() as u32 * CELL;
        draw_text(
            rgba,
            canvas,
            font,
            text,
            (right.saturating_sub(width), y),
            RULE_WHITE,
        );
    }
}

/// Where the two tabs' shared centre wants to be: the canvas centre when
/// the column is collapsed, the visible map's centre when it is open.
pub fn tab_centre_wanted(canvas: &Canvas, open: bool) -> u32 {
    if open {
        (COLUMN_CLEAR_X + canvas.w - SIDE_BORDER) / 2
    } else {
        canvas.w / 2
    }
}

/// The shared centre of the sky and wind tabs (2.2.6). Text never sits
/// under a tab: on a narrow canvas the centre stops short of the ribbon's
/// columns on its left and of the right-aligned counters and date. The
/// ribbon's columns count whether or not a member fills them, so the tab
/// never moves when the party changes.
pub fn tab_centre(canvas: &Canvas, open: bool, sky_width: u32, ribbon: &RibbonSnapshot) -> u32 {
    let wanted = tab_centre_wanted(canvas, open);
    let half = sky_width / 2;
    let layout = ribbon_layout(canvas, open);
    let member_w = layout.member_cells() as u32 * CELL;
    let lo = layout
        .columns
        .iter()
        .filter(|x| **x < wanted)
        .map(|x| x + member_w + half)
        .max()
        .unwrap_or(0);
    let right_text = layout
        .columns
        .iter()
        .filter(|x| **x >= wanted)
        .copied()
        .chain([counters_left(canvas, ribbon)])
        .min()
        .unwrap_or(canvas.w);
    let hi = right_text.saturating_sub(half);
    // Both limits cannot hold on a canvas too narrow for the text: then the
    // right-hand text wins.
    wanted.clamp(lo.min(hi), hi)
}

/// Copy the frame rectangle `rect` to the canvas with its left edge at
/// `x0` and its top at `y0`, clipped to the canvas.
fn copy_rect(
    rgba: &mut [u8],
    canvas: &Canvas,
    faithful: &[u8],
    rect: GapPixelRect,
    (x0, y0): (u32, u32),
) {
    let frame_w = crate::VISUAL_PLAY_FRAME_WIDTH as usize;
    let w = canvas.w as usize;
    for row in 0..=(rect.y1 - rect.y0) {
        let y = y0 as usize + row;
        if y >= canvas.h as usize {
            break;
        }
        let width = rect.width().min(w.saturating_sub(x0 as usize));
        let from = ((rect.y0 + row) * frame_w + rect.x0) * 4;
        let at = (y * w + x0 as usize) * 4;
        rgba[at..at + width * 4].copy_from_slice(&faithful[from..from + width * 4]);
    }
}

/// Where each tab lands: its left edge on the canvas, so that its own
/// measured extent is centred on `centre`.
pub fn tab_left(rect: GapPixelRect, centre: u32) -> u32 {
    centre.saturating_sub(rect.width() as u32 / 2)
}

/// Step 6: the sky tab (frame row 0, caps included) on canvas rows 8..15
/// and the wind tab (frame row 23) on the bottom band, both pixel crops of
/// the faithful frame, centred on one shared centre.
pub fn paint_tabs(
    rgba: &mut [u8],
    canvas: &Canvas,
    faithful: &[u8],
    gaps: &ChromeGapExtents,
    centre: u32,
) {
    if let Some(rect) = gaps.top_rect() {
        copy_rect(rgba, canvas, faithful, rect, (tab_left(rect, centre), CELL));
    }
    if let Some(rect) = gaps.bottom_rect() {
        copy_rect(
            rgba,
            canvas,
            faithful,
            rect,
            (tab_left(rect, centre), canvas.h - CELL),
        );
    }
}

/// Steps 5 and 6 together, with the shared centre worked out from the
/// sky tab's width (or the strip's when there is none).
pub fn paint_bands(
    rgba: &mut [u8],
    canvas: &Canvas,
    faithful: &[u8],
    bands: &BandSnapshot,
    font: &FixedCellFont,
    hp_colours: bool,
    open: bool,
) {
    paint_ribbon(rgba, canvas, &bands.ribbon, font, hp_colours, open);
    let sky_width = bands
        .gaps
        .top_rect()
        .map_or(TAB_STRIP - 2 * CELL, |rect| rect.width() as u32);
    let centre = tab_centre(canvas, open, sky_width, &bands.ribbon);
    paint_tabs(rgba, canvas, faithful, &bands.gaps, centre);
}

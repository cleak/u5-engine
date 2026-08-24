//! Telescope/spyglass sky overlay from `systems/view.md §4.2`.

use crate::{GameClock, TileGraphicsDepth, TileViewport, ui_colour_slot, ui_colour_slot_bright};

pub const SKY_VIEW_ROWS: usize = 8;
pub const SKY_VIEW_COLUMNS: usize = 22;
pub const SKY_VIEW_STARS: usize = 80;
pub const SKY_VIEW_PIXEL_SIDE: usize = 176;
pub const TELESCOPE_LOOK_TRIGGER_TILE: u8 = 0x59;

const SKY_VIEW_SCREEN_ORIGIN: usize = 8;
const SKY_VIEW_SCREEN_END: usize = 183;
const SKY_EPOCH_YEAR_MOD_100: usize = 39;
const SKY_EPOCH_MONTH: usize = 4;
const SKY_EPOCH_DAY: usize = 5;
const SKY_DAYS_PER_MONTH: usize = 28;
const SKY_DAYS_PER_YEAR: usize = 364;
const SKY_YEAR_CYCLE_DAYS: usize = 100 * SKY_DAYS_PER_YEAR;

const ROW_0: [u8; 3] = [4, 11, 18];
const ROW_1: [u8; 5] = [2, 7, 11, 15, 20];
const ROW_2: [u8; 7] = [2, 5, 8, 11, 14, 17, 20];
const ROW_3: [u8; 11] = [1, 3, 5, 7, 9, 11, 13, 15, 17, 19, 21];
const ROW_4: [u8; 13] = [0, 2, 4, 6, 8, 9, 11, 13, 14, 16, 18, 19, 21];
const ROW_5: [u8; 17] = [1, 2, 3, 5, 6, 7, 9, 10, 11, 12, 13, 15, 16, 17, 19, 20, 21];
const ROW_6: [u8; 19] = [
    1, 2, 3, 4, 5, 6, 8, 9, 10, 11, 12, 13, 14, 16, 17, 18, 19, 20, 21,
];
const ROW_7: [u8; 22] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SkyRowSpec {
    pub y_origin: u8,
    pub start_column: u8,
    pub permitted_columns: &'static [u8],
}

pub const SKY_ROW_SPECS: [SkyRowSpec; SKY_VIEW_ROWS] = [
    SkyRowSpec {
        y_origin: 144,
        start_column: 18,
        permitted_columns: &ROW_0,
    },
    SkyRowSpec {
        y_origin: 136,
        start_column: 2,
        permitted_columns: &ROW_1,
    },
    SkyRowSpec {
        y_origin: 120,
        start_column: 8,
        permitted_columns: &ROW_2,
    },
    SkyRowSpec {
        y_origin: 104,
        start_column: 15,
        permitted_columns: &ROW_3,
    },
    SkyRowSpec {
        y_origin: 88,
        start_column: 11,
        permitted_columns: &ROW_4,
    },
    SkyRowSpec {
        y_origin: 64,
        start_column: 6,
        permitted_columns: &ROW_5,
    },
    SkyRowSpec {
        y_origin: 40,
        start_column: 4,
        permitted_columns: &ROW_6,
    },
    SkyRowSpec {
        y_origin: 8,
        start_column: 2,
        permitted_columns: &ROW_7,
    },
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SkyOverlayState {
    /// Absolute gameplay-screen coordinates, in the order the PRNG produced them.
    pub stars: [(u8, u8); SKY_VIEW_STARS],
    pub body_columns: [u8; SKY_VIEW_ROWS],
}

pub const fn sky_view_is_daylight(hour: u8) -> bool {
    hour >= 6 && hour <= 17
}

/// Calendar days since year `39`, month 4, day 5, with the saved year compared modulo 100.
pub const fn sky_elapsed_days(clock: GameClock) -> usize {
    let current = (clock.year as usize % 100) * SKY_DAYS_PER_YEAR
        + (clock.month as usize - 1) * SKY_DAYS_PER_MONTH
        + (clock.day as usize - 1);
    let epoch = SKY_EPOCH_YEAR_MOD_100 * SKY_DAYS_PER_YEAR
        + (SKY_EPOCH_MONTH - 1) * SKY_DAYS_PER_MONTH
        + (SKY_EPOCH_DAY - 1);
    (current + SKY_YEAR_CYCLE_DAYS - epoch) % SKY_YEAR_CYCLE_DAYS
}

pub fn sky_body_columns(clock: GameClock) -> [u8; SKY_VIEW_ROWS] {
    let elapsed = sky_elapsed_days(clock);
    std::array::from_fn(|row| {
        let spec = SKY_ROW_SPECS[row];
        let start = spec
            .permitted_columns
            .iter()
            .position(|column| *column == spec.start_column)
            .expect("published sky start column belongs to its permitted ring");
        let len = spec.permitted_columns.len();
        spec.permitted_columns[(start + len - elapsed % len) % len]
    })
}

pub fn sky_text_map(state: &SkyOverlayState, shadowlord_hideouts: [u8; 3]) -> String {
    let mut out = String::with_capacity(SKY_VIEW_ROWS * (SKY_VIEW_COLUMNS + 1));
    for (row, column) in state.body_columns.iter().copied().enumerate() {
        let marked = shadowlord_hideouts
            .iter()
            .any(|hideout| *hideout == row as u8 + 1);
        for candidate in 0..SKY_VIEW_COLUMNS as u8 {
            out.push(if candidate == column {
                if marked { '*' } else { 'o' }
            } else {
                ' '
            });
        }
        out.push('\n');
    }
    out
}

pub fn render_sky_overlay(
    depth: TileGraphicsDepth,
    state: &SkyOverlayState,
    shadowlord_hideouts: [u8; 3],
) -> TileViewport {
    let high_colour = depth.pixel_limit() > 4;
    let star_colour = ui_colour_slot_bright(2, high_colour);
    let body_colour = ui_colour_slot(1, high_colour);
    let marker_colour = ui_colour_slot(0, high_colour);
    let mut viewport = TileViewport {
        depth,
        cells_wide: SKY_VIEW_COLUMNS,
        cells_high: SKY_VIEW_ROWS,
        width: SKY_VIEW_PIXEL_SIDE,
        height: SKY_VIEW_PIXEL_SIDE,
        pixels: vec![0; SKY_VIEW_PIXEL_SIDE * SKY_VIEW_PIXEL_SIDE],
    };

    for &(x, y) in &state.stars {
        plot_screen_pixel(&mut viewport, usize::from(x), usize::from(y), star_colour);
    }

    for (row, column) in state.body_columns.iter().copied().enumerate() {
        let y = usize::from(SKY_ROW_SPECS[row].y_origin);
        let x = (usize::from(column) + 1) * 8;
        plot_screen_pixel(&mut viewport, x + 6, y + 8, body_colour);
        for i in 0..3 {
            for j in 0..3 {
                if x + i <= 176 {
                    plot_screen_pixel(&mut viewport, x + 7 + i, y + 7 + j, body_colour);
                }
            }
        }
        if x <= 173 {
            plot_screen_pixel(&mut viewport, x + 10, y + 8, body_colour);
        }

        for hideout in shadowlord_hideouts {
            if hideout == row as u8 + 1 {
                draw_shadowlord_marker(&mut viewport, usize::from(column) * 8, y, marker_colour);
            }
        }
    }
    viewport
}

fn draw_shadowlord_marker(viewport: &mut TileViewport, x: usize, y: usize, colour: u8) {
    let runs = [
        (5, 10, 12, x > 2),
        (6, 10, 12, x > 2),
        (7, 8, 12, x > 2),
        (8, 8, 12, x <= 175),
        (9, 6, 10, x <= 174),
        (10, 6, 10, x <= 173),
        (11, 5, 8, x <= 172),
        (12, 5, 7, x <= 171),
    ];
    for (dx, y0, y1, admitted) in runs {
        if admitted {
            for dy in y0..=y1 {
                plot_screen_pixel(viewport, x + dx, y + dy, colour);
            }
        }
    }
}

fn plot_screen_pixel(viewport: &mut TileViewport, x: usize, y: usize, colour: u8) {
    if !(SKY_VIEW_SCREEN_ORIGIN..=SKY_VIEW_SCREEN_END).contains(&x)
        || !(SKY_VIEW_SCREEN_ORIGIN..=SKY_VIEW_SCREEN_END).contains(&y)
    {
        return;
    }
    let local_x = x - SKY_VIEW_SCREEN_ORIGIN;
    let local_y = y - SKY_VIEW_SCREEN_ORIGIN;
    viewport.pixels[local_y * viewport.width + local_x] = colour % viewport.depth.pixel_limit();
}

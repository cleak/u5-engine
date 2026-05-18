//! Fixed status/stats panel renderer.

use crate::*;

pub const STATS_PANEL_WIDTH: usize = 16;
pub const STATS_PANEL_PARTY_ROWS: usize = SAVE_PARTY_SIZE_MAX as usize;

pub fn render_stats_panel(state: &PlayState, active_cursor: Option<usize>) -> String {
    let mut lines = Vec::with_capacity(STATS_PANEL_PARTY_ROWS + 5);
    lines.push(fixed_panel_line("STATS"));
    for index in 0..STATS_PANEL_PARTY_ROWS {
        lines.push(render_stats_panel_party_row(state, active_cursor, index));
    }
    lines.push(render_stats_panel_food_row(state.food));
    lines.push(render_stats_panel_middle_row(state));
    lines.push(render_stats_panel_date_row(&state.clock));
    lines.push(render_stats_panel_sky_status_row(state));
    lines.join("\n")
}

pub fn stats_panel_active_cursor_visible(state: &PlayState, active_cursor: Option<usize>) -> bool {
    let Some(index) = active_cursor else {
        return false;
    };
    state
        .party
        .get(index)
        .copied()
        .is_some_and(|member| !matches!(member.status, b'D' | b'S'))
}

fn render_stats_panel_party_row(
    state: &PlayState,
    active_cursor: Option<usize>,
    index: usize,
) -> String {
    let Some(member) = state.party.get(index).copied() else {
        return " ".repeat(STATS_PANEL_WIDTH);
    };
    let name = state
        .party_names
        .get(index)
        .and_then(|name| party_name_to_string(name))
        .unwrap_or_else(|| format!("Party {}", index + 1));
    let name = truncate_ascii_chars(&name, 10);
    let cursor = if active_cursor == Some(index) && !matches!(member.status, b'D' | b'S') {
        '>'
    } else {
        ' '
    };
    fixed_panel_line(&format!(
        "{name:<10}{cursor}{:>4}{}",
        member.hp.min(9999),
        char::from(member.status)
    ))
}

fn render_stats_panel_food_row(food: u16) -> String {
    fixed_panel_line(&format!("Food{:>12}", food.min(9999)))
}

fn render_stats_panel_middle_row(state: &PlayState) -> String {
    match stats_panel_middle_counter(state.player.transport.save_marker()) {
        StatsPanelMiddleCounter::PartyGold => fixed_panel_line(&format!("Gold{:>12}", state.gold)),
        StatsPanelMiddleCounter::ShipHullCondition => fixed_panel_line(&format!(
            "Ship hull{:>7}",
            current_ship_hull(state).unwrap_or(0)
        )),
    }
}

fn render_stats_panel_date_row(clock: &GameClock) -> String {
    fixed_panel_line(&format!(
        "Date {:02}-{:02} {:03}",
        clock.month, clock.day, clock.year
    ))
}

fn render_stats_panel_sky_status_row(state: &PlayState) -> String {
    let glyph = state.timing_status.save_byte();
    let glyph = if glyph == 0 { '-' } else { char::from(glyph) };
    match state.area {
        Area::Dungeon { .. } => fixed_panel_line(&format!(
            "Light T{:>3} S{:>3}",
            state.torch_counter, state.light_spell_counter
        )),
        Area::World { plane } if plane == WorldPlane::Underworld => {
            fixed_panel_line(&format!("Underworld  {glyph}"))
        }
        Area::World { .. } | Area::Town { .. } => {
            fixed_panel_line(&format!("Sky {} {glyph}", sky_strip_text(state.clock.hour)))
        }
    }
}

fn current_ship_hull(state: &PlayState) -> Option<u8> {
    match state.player.transport {
        TransportState::Ship { hull, .. } => Some(hull),
        _ => None,
    }
}

fn sky_strip_text(hour: u8) -> String {
    sky_strip_composed_cells(hour)
        .into_iter()
        .map(|cell| match cell {
            Some(SkyStripMarker::FixedHour) => '|',
            Some(SkyStripMarker::Trammel) => 'T',
            Some(SkyStripMarker::Felucca) => 'F',
            None => '.',
        })
        .collect()
}

fn truncate_ascii_chars(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

fn fixed_panel_line(value: &str) -> String {
    let mut line = truncate_ascii_chars(value, STATS_PANEL_WIDTH);
    while line.chars().count() < STATS_PANEL_WIDTH {
        line.push(' ');
    }
    line
}

//! Fixed status/stats panel renderer.

use crate::*;

pub const STATS_PANEL_WIDTH: usize = 16;
pub const STATS_PANEL_PARTY_ROWS: usize = SAVE_PARTY_SIZE_MAX as usize;
pub const MAIN_TEXT_WINDOW_INDEX: usize = 0;
pub const STATS_PANEL_TEXT_WINDOW_INDEX: usize = 1;
pub const TALK_SHOP_TEXT_WINDOW_INDEX: usize = 2;
pub const PROMPT_TEXT_WINDOW_INDEX: usize = 3;
pub const MESSAGE_TEXT_WINDOW_RIGHT: u8 = 23;
pub const STATS_PANEL_TEXT_LEFT: u8 = 23;
pub const STATS_PANEL_TEXT_RIGHT: u8 = TEXT_SCREEN_COLUMNS - 1;
pub const STATS_PANEL_TEXT_BOTTOM: u8 = TEXT_SCREEN_ROWS - 1;
pub const INN_PICKUP_REGISTER_TEXT_WINDOW_INDEX: usize = STATS_PANEL_TEXT_WINDOW_INDEX;
pub const INN_PICKUP_REGISTER_LEFT: u8 = 24;
pub const INN_PICKUP_REGISTER_TOP: u8 = 1;
pub const INN_PICKUP_REGISTER_INITIAL_RIGHT: u8 = 38;
pub const INN_PICKUP_REGISTER_FRAME_RIGHT: u8 = 39;
pub const INN_PICKUP_REGISTER_BOTTOM: u8 = 9;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StatsPanelCombatRowOverlay {
    pub highlighted: bool,
    pub status_override: Option<u8>,
}

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

pub fn configure_play_text_windows(system: &mut TextWindowSystem) {
    system.set_window_rect(
        MAIN_TEXT_WINDOW_INDEX,
        0,
        0,
        MESSAGE_TEXT_WINDOW_RIGHT,
        TEXT_SCREEN_ROWS - 1,
    );
    system.set_window_rect(
        STATS_PANEL_TEXT_WINDOW_INDEX,
        STATS_PANEL_TEXT_LEFT,
        0,
        STATS_PANEL_TEXT_RIGHT,
        STATS_PANEL_TEXT_BOTTOM,
    );
    system.set_window_rect(
        PROMPT_TEXT_WINDOW_INDEX,
        0,
        TEXT_SCREEN_ROWS - 2,
        MESSAGE_TEXT_WINDOW_RIGHT,
        TEXT_SCREEN_ROWS - 1,
    );
    system.set_active_window(MAIN_TEXT_WINDOW_INDEX);
}

pub fn configure_talk_shop_text_window(system: &mut TextWindowSystem) {
    system.set_window_rect(
        TALK_SHOP_TEXT_WINDOW_INDEX,
        0,
        0,
        MESSAGE_TEXT_WINDOW_RIGHT,
        TEXT_SCREEN_ROWS - 1,
    );
    system.set_active_window(TALK_SHOP_TEXT_WINDOW_INDEX);
}

pub fn render_play_text_window_system(
    state: &PlayState,
    active_cursor: Option<usize>,
    input_echo: Option<&str>,
) -> TextWindowSystem {
    let mut system = TextWindowSystem::new();
    configure_play_text_windows(&mut system);
    let message = state
        .active_shop
        .as_ref()
        .map(|shop| shop.modal_text(&state.message))
        .unwrap_or_else(|| state.message.clone());
    if state.active_shop.is_some() {
        configure_talk_shop_text_window(&mut system);
        paint_talk_shop_text_window(&mut system, &message);
    } else {
        paint_message_text_window(&mut system, &message);
    }
    paint_stats_panel_text_window(&mut system, state, active_cursor);
    if state.active_shop.is_some() {
        paint_inn_pickup_register_text_window(&mut system, state);
    }
    if let Some(input_echo) = input_echo {
        paint_prompt_text_window(&mut system, input_echo);
    }
    if state.active_shop.is_some() {
        system.set_active_window(TALK_SHOP_TEXT_WINDOW_INDEX);
    } else {
        system.set_active_window(MAIN_TEXT_WINDOW_INDEX);
    }
    system
}

pub fn render_play_text_window_ascii(
    state: &PlayState,
    active_cursor: Option<usize>,
    input_echo: Option<&str>,
) -> String {
    render_play_text_window_system(state, active_cursor, input_echo)
        .screen_rows(b' ')
        .join("\n")
}

pub fn paint_message_text_window(system: &mut TextWindowSystem, message: &str) {
    system.set_active_window(MAIN_TEXT_WINDOW_INDEX);
    system.emit_byte(TEXT_CTRL_CLEAR_WINDOW);
    system.set_active_cursor(0, 0);
    system.print_wrapped_string(message);
}

pub fn paint_talk_shop_text_window(system: &mut TextWindowSystem, message: &str) {
    system.set_active_window(TALK_SHOP_TEXT_WINDOW_INDEX);
    system.emit_byte(b'\r');
    system.emit_byte(b'\n');
    system.print_wrapped_string(message);
    system.set_active_window(TALK_SHOP_TEXT_WINDOW_INDEX);
}

pub fn paint_inn_pickup_register_text_window(system: &mut TextWindowSystem, state: &PlayState) {
    let Some(crate::shop_session::ActiveShopSession::Innkeeper(
        crate::shop_runtime::InnkeeperState::PickUpCompanion {
            guest_indices,
            guest_count,
            ..
        },
    )) = state.active_shop.as_ref()
    else {
        return;
    };

    system.set_active_window(INN_PICKUP_REGISTER_TEXT_WINDOW_INDEX);
    system.set_window_rect(
        INN_PICKUP_REGISTER_TEXT_WINDOW_INDEX,
        INN_PICKUP_REGISTER_LEFT,
        INN_PICKUP_REGISTER_TOP,
        INN_PICKUP_REGISTER_INITIAL_RIGHT,
        INN_PICKUP_REGISTER_BOTTOM,
    );
    system.emit_byte(TEXT_CTRL_CLEAR_WINDOW);
    system.set_window_rect(
        INN_PICKUP_REGISTER_TEXT_WINDOW_INDEX,
        INN_PICKUP_REGISTER_LEFT,
        INN_PICKUP_REGISTER_TOP,
        INN_PICKUP_REGISTER_FRAME_RIGHT,
        INN_PICKUP_REGISTER_BOTTOM,
    );

    system.set_active_cursor(1, 1);
    system.print_wrapped_string("Pick up");
    system.set_active_cursor(1, 2);
    system.print_wrapped_string("Companion");

    let rows = usize::from(*guest_count).min(INN_REGISTRY_CAP).min(5);
    for row in 0..rows {
        let Some(guest) = guest_indices
            .get(row)
            .and_then(|index| state.inn_registry.get(*index))
        else {
            continue;
        };
        let display_row = 3 + row;
        let name =
            party_name_to_string(&guest.name).unwrap_or_else(|| format!("Guest {}", row + 1));
        let line = format!("{} {}", row + 1, truncate_ascii_chars(&name, 11));
        system.set_active_cursor(1, display_row.min(u8::MAX as usize) as u8);
        system.print_wrapped_string(&line);
    }

    system.set_active_window(TALK_SHOP_TEXT_WINDOW_INDEX);
}

pub fn paint_stats_panel_text_window(
    system: &mut TextWindowSystem,
    state: &PlayState,
    active_cursor: Option<usize>,
) {
    system.set_active_window(STATS_PANEL_TEXT_WINDOW_INDEX);
    system.emit_byte(TEXT_CTRL_CLEAR_WINDOW);
    system.clear_active_flags();

    system.set_active_cursor(0, 0);
    emit_fixed_panel_line(system, &fixed_panel_line("STATS"));

    for index in 0..STATS_PANEL_PARTY_ROWS {
        let row = index + 1;
        system.set_active_cursor(0, row.min(u8::MAX as usize) as u8);
        paint_stats_panel_party_row(system, state, active_cursor, index);
    }

    let bottom_rows = [
        render_stats_panel_food_row(state.food),
        render_stats_panel_middle_row(state),
        render_stats_panel_date_row(&state.clock),
        render_stats_panel_sky_status_row(state),
    ];
    for (offset, line) in bottom_rows.iter().enumerate() {
        let row = STATS_PANEL_PARTY_ROWS + 1 + offset;
        if row >= usize::from(TEXT_SCREEN_ROWS) {
            break;
        }
        system.set_active_cursor(0, row.min(u8::MAX as usize) as u8);
        emit_fixed_panel_line(system, line);
    }
}

pub fn paint_prompt_text_window(system: &mut TextWindowSystem, input_echo: &str) {
    paint_prompt_text_window_with_cursor(system, input_echo, None);
}

pub fn paint_prompt_text_window_with_cursor(
    system: &mut TextWindowSystem,
    input_echo: &str,
    cursor_glyph: Option<u8>,
) {
    system.set_active_window(PROMPT_TEXT_WINDOW_INDEX);
    system.emit_byte(TEXT_CTRL_CLEAR_WINDOW);
    system.set_active_cursor(0, 0);
    system.emit_byte(b'>');
    system.emit_byte(b' ');
    system.print_wrapped_string(input_echo);
    if let Some(cursor_glyph) = cursor_glyph {
        system.paint_cursor_glyph(cursor_glyph);
    }
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
    stats_panel_party_row(state, active_cursor, index).0
}

fn stats_panel_party_row(
    state: &PlayState,
    active_cursor: Option<usize>,
    index: usize,
) -> (String, StatsPanelCombatRowOverlay) {
    let Some(member) = state.party.get(index).copied() else {
        return (
            " ".repeat(STATS_PANEL_WIDTH),
            StatsPanelCombatRowOverlay::default(),
        );
    };
    let name = state
        .party_names
        .get(index)
        .and_then(|name| party_name_to_string(name))
        .unwrap_or_else(|| format!("Party {}", index + 1));
    let name = truncate_ascii_chars(&name, 10);
    let overlay = stats_panel_combat_row_overlay(state, index);
    let cursor = if active_cursor == Some(index) && !matches!(member.status, b'D' | b'S') {
        '>'
    } else {
        ' '
    };
    let status = overlay.status_override.unwrap_or(member.status);
    (
        fixed_panel_line(&format!(
            "{name:<10}{cursor}{:>4}{}",
            member.hp.min(9999),
            char::from(status)
        )),
        overlay,
    )
}

fn paint_stats_panel_party_row(
    system: &mut TextWindowSystem,
    state: &PlayState,
    active_cursor: Option<usize>,
    index: usize,
) {
    let (line, overlay) = stats_panel_party_row(state, active_cursor, index);
    if overlay.highlighted {
        system.emit_byte(TEXT_CTRL_INVERSE_TOGGLE);
    }
    emit_fixed_panel_line(system, &line);
    if overlay.highlighted {
        system.emit_byte(TEXT_CTRL_INVERSE_TOGGLE);
    }
}

fn emit_fixed_panel_line(system: &mut TextWindowSystem, line: &str) {
    for byte in fixed_panel_line(line).bytes().take(STATS_PANEL_WIDTH) {
        system.emit_byte(byte);
    }
}

pub fn stats_panel_combat_row_overlay(
    state: &PlayState,
    index: usize,
) -> StatsPanelCombatRowOverlay {
    if !state.combat_active || index >= STATS_PANEL_PARTY_ROWS {
        return StatsPanelCombatRowOverlay::default();
    }

    let highlighted = state
        .pending_combat_actor_slot
        .filter(|slot| *slot < COMBAT_PARTY_ACTOR_SLOTS)
        .and_then(|slot| state.combat_actors.get(slot).copied())
        .is_some_and(|actor| {
            usize::from(actor.owner_target_class) == index && combat_actor_is_active_not_dead(actor)
        });

    let status_override = stats_panel_combat_cast_status_override(state, index);
    StatsPanelCombatRowOverlay {
        highlighted,
        status_override,
    }
}

fn stats_panel_combat_cast_status_override(state: &PlayState, index: usize) -> Option<u8> {
    if state
        .active_cast
        .as_ref()
        .is_some_and(|session| session.combat_actor_slot == Some(index))
        || state
            .active_cast_followup
            .as_ref()
            .is_some_and(|session| session.combat_actor_slot == Some(index))
    {
        Some(b'C')
    } else {
        None
    }
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
        Area::World { .. } | Area::Town { .. } => fixed_panel_line(&format!(
            "Sky {} {glyph}",
            sky_strip_text(state.clock.hour, state.cached_moon_glyph_bytes)
        )),
    }
}

fn current_ship_hull(state: &PlayState) -> Option<u8> {
    match state.player.transport {
        TransportState::Ship { hull, .. } => Some(hull),
        _ => None,
    }
}

fn sky_strip_text(hour: u8, cached_moon_glyph_bytes: [u8; 2]) -> String {
    sky_strip_composed_cells(hour)
        .into_iter()
        .map(|cell| match cell {
            Some(SkyStripMarker::FixedHour) => '|',
            Some(SkyStripMarker::Trammel) => moon_glyph_cell(cached_moon_glyph_bytes[0]),
            Some(SkyStripMarker::Felucca) => moon_glyph_cell(cached_moon_glyph_bytes[1]),
            None => '.',
        })
        .collect()
}

fn moon_glyph_cell(byte: u8) -> char {
    if (b'0'..=b'7').contains(&byte) {
        char::from(byte)
    } else {
        '-'
    }
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

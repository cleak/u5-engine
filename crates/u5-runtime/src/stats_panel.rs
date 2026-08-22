//! Fixed status/stats panel renderer.

use crate::*;

/// Stats-panel text area width in cells: columns 24..=38, bounded by
/// the white rules at `x=191` and `x=312`. See `gameplay_chrome` for
/// the geometry's provenance and the pending spec question.
pub const STATS_PANEL_WIDTH: usize = 15;
pub const STATS_PANEL_PARTY_ROWS: usize = SAVE_PARTY_SIZE_MAX as usize;
pub const MAIN_TEXT_WINDOW_INDEX: usize = 0;
pub const STATS_PANEL_TEXT_WINDOW_INDEX: usize = 1;
pub const TALK_SHOP_TEXT_WINDOW_INDEX: usize = 2;
pub const PROMPT_TEXT_WINDOW_INDEX: usize = 3;
pub const MESSAGE_TEXT_WINDOW_RIGHT: u8 = MESSAGE_WINDOW_RIGHT;
pub const STATS_PANEL_TEXT_LEFT: u8 = 24;
/// `text-output.md §4`: a window's printable width is
/// `bottom_right_x - top_left_x`, excluding the trailing column. The
/// right edge therefore sits one column past the last painted cell so
/// all fifteen cells (columns 24..=38) are printable.
pub const STATS_PANEL_TEXT_RIGHT: u8 = STATS_PANEL_TEXT_LEFT + STATS_PANEL_WIDTH as u8;
/// Roster rows 1..=6, then the divider band at row 7, then the
/// food/gold and calendar rows 8..=9. Row 7 is chrome and is never
/// written by the panel.
pub const STATS_PANEL_TEXT_TOP: u8 = STATS_ROSTER_TOP;
/// One row past the last painted row: emitting a full-width line
/// wraps the cursor onto the following row, and without the spare row
/// that wrap would scroll the window and shift every line up by one.
pub const STATS_PANEL_TEXT_BOTTOM: u8 = STATS_COUNTER_BOTTOM + 1;
/// Cells of the party row's left-aligned name field.
pub const STATS_PANEL_NAME_CELLS: usize = 9;
/// Cells of the party row's right-justified hit-point field.
pub const STATS_PANEL_HP_CELLS: usize = 4;
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
    let mut lines = Vec::with_capacity(STATS_PANEL_PARTY_ROWS + 2);
    for index in 0..STATS_PANEL_PARTY_ROWS {
        lines.push(render_stats_panel_party_row(state, active_cursor, index));
    }
    lines.push(render_stats_panel_counter_row(state));
    lines.push(render_stats_panel_date_row(&state.clock));
    lines.join("\n")
}

/// Lay out the gameplay screen's text windows. The message/command
/// window is the right-hand column below the stats boxes, and the live
/// input line is its own bottom row rather than a separate bottom-left
/// prompt window. Text row 24 is never covered by any window.
pub fn configure_play_text_windows(system: &mut TextWindowSystem) {
    system.set_window_rect(
        MAIN_TEXT_WINDOW_INDEX,
        MESSAGE_WINDOW_LEFT,
        MESSAGE_WINDOW_TOP,
        MESSAGE_WINDOW_RIGHT,
        MESSAGE_WINDOW_BOTTOM,
    );
    system.set_window_rect(
        STATS_PANEL_TEXT_WINDOW_INDEX,
        STATS_PANEL_TEXT_LEFT,
        STATS_PANEL_TEXT_TOP,
        STATS_PANEL_TEXT_RIGHT,
        STATS_PANEL_TEXT_BOTTOM,
    );
    system.set_window_rect(
        PROMPT_TEXT_WINDOW_INDEX,
        MESSAGE_WINDOW_LEFT,
        MESSAGE_WINDOW_BOTTOM,
        MESSAGE_WINDOW_RIGHT,
        MESSAGE_WINDOW_BOTTOM,
    );
    system.set_active_window(MAIN_TEXT_WINDOW_INDEX);
}

pub fn configure_talk_shop_text_window(system: &mut TextWindowSystem) {
    system.set_window_rect(
        TALK_SHOP_TEXT_WINDOW_INDEX,
        MESSAGE_WINDOW_LEFT,
        MESSAGE_WINDOW_TOP,
        MESSAGE_WINDOW_RIGHT,
        MESSAGE_WINDOW_BOTTOM,
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

    for index in 0..STATS_PANEL_PARTY_ROWS {
        system.set_active_cursor(0, index.min(u8::MAX as usize) as u8);
        paint_stats_panel_party_row(system, state, active_cursor, index);
    }

    // Screen rows 8 and 9 are window rows 7 and 8: the divider band at
    // screen row 7 is chrome, not a window row.
    let counter_row = STATS_COUNTER_TOP - STATS_PANEL_TEXT_TOP;
    let bottom_rows = [
        render_stats_panel_counter_row(state),
        render_stats_panel_date_row(&state.clock),
    ];
    for (offset, line) in bottom_rows.iter().enumerate() {
        system.set_active_cursor(0, counter_row + offset as u8);
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
    // Column 24 carries the two-colour ribbon end-cap sprite, painted
    // by the chrome pass; the echoed text starts one column in.
    system.set_active_cursor(1, 0);
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
    let name = truncate_ascii_chars(&name, STATS_PANEL_NAME_CELLS);
    let overlay = stats_panel_combat_row_overlay(state, index);
    let cursor = if active_cursor == Some(index) && !matches!(member.status, b'D' | b'S') {
        '>'
    } else {
        ' '
    };
    let status = overlay.status_override.unwrap_or(member.status);
    (
        fixed_panel_line(&format!(
            "{name:<STATS_PANEL_NAME_CELLS$}{cursor}{:>STATS_PANEL_HP_CELLS$}{}",
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

/// Screen row 8: provisions left-aligned at column 24 and the middle
/// counter right-aligned to end at column 38, sharing one row.
fn render_stats_panel_counter_row(state: &PlayState) -> String {
    let food = format!("F:{}", state.food.min(9999));
    let middle = render_stats_panel_middle_counter(state);
    let used = food.chars().count() + middle.chars().count();
    let padding = STATS_PANEL_WIDTH.saturating_sub(used);
    fixed_panel_line(&format!("{food}{}{middle}", " ".repeat(padding)))
}

fn render_stats_panel_middle_counter(state: &PlayState) -> String {
    match stats_panel_middle_counter(state.player.transport.save_marker()) {
        StatsPanelMiddleCounter::PartyGold => format!("G:{}", state.gold),
        StatsPanelMiddleCounter::ShipHullCondition => {
            format!("H:{}", current_ship_hull(state).unwrap_or(0))
        }
    }
}

/// Screen row 9: the calendar, centred in the fifteen-column text
/// area. `stats-panel.md §5` gives "a short M-D pair" and "the year
/// printed as a three-digit zero-padded value".
fn render_stats_panel_date_row(clock: &GameClock) -> String {
    centred_panel_line(&format!("{}-{}-{:03}", clock.month, clock.day, clock.year))
}

fn current_ship_hull(state: &PlayState) -> Option<u8> {
    match state.player.transport {
        TransportState::Ship { hull, .. } => Some(hull),
        _ => None,
    }
}

fn truncate_ascii_chars(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

fn centred_panel_line(value: &str) -> String {
    let value = truncate_ascii_chars(value, STATS_PANEL_WIDTH);
    let leading = (STATS_PANEL_WIDTH - value.chars().count()) / 2;
    fixed_panel_line(&format!("{}{value}", " ".repeat(leading)))
}

fn fixed_panel_line(value: &str) -> String {
    let mut line = truncate_ascii_chars(value, STATS_PANEL_WIDTH);
    while line.chars().count() < STATS_PANEL_WIDTH {
        line.push(' ');
    }
    line
}

//! Fixed status/stats panel renderer.

use crate::*;

/// Stats-panel text area width in cells: columns 24..=38, bounded by
/// the white rules at `x=191` and `x=312` (`text-output.md §10.1`).
pub const STATS_PANEL_WIDTH: usize = 15;
pub const STATS_PANEL_PARTY_ROWS: usize = SAVE_PARTY_SIZE_MAX as usize;
/// Full-screen window used by the standing chrome writers.
pub const FULL_SCREEN_TEXT_WINDOW_INDEX: usize = 0;
pub const STATS_PANEL_TEXT_WINDOW_INDEX: usize = 1;
/// Standing gameplay message window: command echoes, output, Talk/shop
/// dialogue, and the live prompt line all share this descriptor.
pub const MESSAGE_TEXT_WINDOW_INDEX: usize = 2;
/// Compatibility name retained for callers that describe ordinary gameplay
/// output as the main text stream.
pub const MAIN_TEXT_WINDOW_INDEX: usize = MESSAGE_TEXT_WINDOW_INDEX;
pub const TALK_SHOP_TEXT_WINDOW_INDEX: usize = MESSAGE_TEXT_WINDOW_INDEX;
/// Prompts are not a fourth gameplay window; they use the last row of window 2.
pub const PROMPT_TEXT_WINDOW_INDEX: usize = MESSAGE_TEXT_WINDOW_INDEX;
pub const UNUSED_TEXT_WINDOW_INDEX: usize = 3;
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
/// Published stats-window bottom row (`text-output.md` section 10.1:
/// the window is cells `(24, 1)-(39, 9)`, pixels `(192, 8)-(319, 79)`).
/// The window spans columns 24..=39 but the panel only ever writes
/// 24..=38 - that is why the field is fifteen cells.
pub const STATS_PANEL_TEXT_BOTTOM: u8 = STATS_COUNTER_BOTTOM;
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
pub const INN_PICKUP_REGISTER_BORDER_FIRST_ROW: u8 = 2;
pub const INN_PICKUP_REGISTER_BORDER_LAST_ROW: u8 = 8;
pub const ARMS_SELL_BROWSER_TEXT_WINDOW_INDEX: usize = STATS_PANEL_TEXT_WINDOW_INDEX;
pub const ARMS_SELL_BROWSER_LEFT: u8 = 24;
pub const ARMS_SELL_BROWSER_TOP: u8 = 1;
pub const ARMS_SELL_BROWSER_INITIAL_RIGHT: u8 = 38;
pub const ARMS_SELL_BROWSER_INITIAL_BOTTOM: u8 = 6;
pub const ARMS_SELL_BROWSER_FRAME_RIGHT: u8 = 39;
pub const ARMS_SELL_BROWSER_FRAME_BOTTOM: u8 = 9;
pub const ARMS_SELL_BROWSER_BORDER_FIRST_ROW: u8 = 2;
pub const ARMS_SELL_BROWSER_BORDER_LAST_ROW: u8 = 5;
pub const ARMS_SELL_BROWSER_PAGE_BADGE_LOCAL_COLUMN: u8 = 6;
pub const ARMS_SELL_BROWSER_PAGE_BADGE_LOCAL_ROW: u8 = 6;
pub const ARMS_SELL_BROWSER_PAGE_BADGE_OPEN: u8 = 0x02;
pub const ARMS_SELL_BROWSER_PAGE_BADGE_CLOSE: u8 = 0x01;
pub const ARMS_SELL_BROWSER_PAGE_GLYPH_DOWN: u8 = 0x19;
pub const ARMS_SELL_BROWSER_PAGE_GLYPH_UP: u8 = 0x18;
pub const ARMS_SELL_BROWSER_PAGE_GLYPH_BOTH: u8 = 0x12;
pub const ARMS_SELL_BROWSER_STATS_LABEL: &str = "Arms";
/// Both shop-owned side panels place their paired bracket caps at
/// window-local columns 0 and 14, i.e. absolute columns 24 and 38.
pub const SHOP_SIDE_PANEL_LEFT_BORDER_COLUMN: u8 = 24;
pub const SHOP_SIDE_PANEL_RIGHT_BORDER_COLUMN: u8 = 38;

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

/// Lay out the gameplay screen's text windows.
///
/// `cleak/u5-spec#62`, resolved. An earlier revision of that issue
/// published a census claiming windows 2 and 3 are never passed to the
/// rectangle setter, so window 2 would keep a boot-time full-screen
/// descriptor. That claim has been withdrawn in three spec documents:
/// the gameplay-screen assembly shapes window 2 once to the
/// message-window rectangle, and shop and conversation text is bounded
/// by that rectangle rather than by the full screen. This engine
/// already did that - the divergence recorded here previously was the
/// engine being right - so the shop and conversation paths cannot spill
/// outside the message window. See `configure_talk_shop_text_window`.
///
/// The message/command
/// window is the right-hand column below the stats boxes, and the live
/// input line is its own bottom row rather than a separate bottom-left
/// prompt window. Text row 24 is never covered by any window.
pub fn configure_play_text_windows(system: &mut TextWindowSystem) {
    // `text-output.md §10.1`: assembly first narrows window 0 over the
    // intro's lower text block, restores its full-screen rectangle, and then
    // clears through that restored descriptor before installing the standing
    // gameplay windows.
    system.set_window_rect(FULL_SCREEN_TEXT_WINDOW_INDEX, 1, 16, 38, 23);
    system.set_window_rect(
        FULL_SCREEN_TEXT_WINDOW_INDEX,
        0,
        0,
        TEXT_SCREEN_COLUMNS - 1,
        TEXT_SCREEN_ROWS - 1,
    );
    system.set_active_window(FULL_SCREEN_TEXT_WINDOW_INDEX);
    system.emit_byte(TEXT_CTRL_CLEAR_WINDOW);
    system.set_window_rect(
        STATS_PANEL_TEXT_WINDOW_INDEX,
        STATS_PANEL_TEXT_LEFT,
        STATS_PANEL_TEXT_TOP,
        STATS_PANEL_TEXT_RIGHT,
        STATS_PANEL_TEXT_BOTTOM,
    );
    system.set_window_rect(
        MESSAGE_TEXT_WINDOW_INDEX,
        MESSAGE_WINDOW_LEFT,
        MESSAGE_WINDOW_TOP,
        MESSAGE_WINDOW_RIGHT,
        MESSAGE_WINDOW_BOTTOM,
    );
    system.set_active_window(MESSAGE_TEXT_WINDOW_INDEX);
    system.set_active_cursor(0, MESSAGE_WINDOW_BOTTOM - MESSAGE_WINDOW_TOP);
}

pub fn configure_talk_shop_text_window(system: &mut TextWindowSystem) {
    // `text-output.md §9`: Talk/shop overlays inherit the standing window-2
    // descriptor and cursor; they do not reshape or home it.
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
        paint_arms_sell_browser_text_window(&mut system, state);
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
    system.set_active_window(MESSAGE_TEXT_WINDOW_INDEX);
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

/// Paint the published arms-shop `S` browser and its four row interiors.
///
/// `shops.md §8.1` fixes the window-1 clear/widen handoff, ascending
/// nonzero equipment rows, fixed short labels, inverse selected row,
/// blank tail rows. The gameplay-chrome pass owns the stats-ribbon `Arms`
/// label and exact three-cell page-status badge so their cap cells retain
/// the shared two-colour ribbon treatment.
pub fn paint_arms_sell_browser_text_window(system: &mut TextWindowSystem, state: &PlayState) {
    let Some(browser) = active_arms_sell_browser(state) else {
        return;
    };

    system.set_active_window(ARMS_SELL_BROWSER_TEXT_WINDOW_INDEX);
    system.set_window_rect(
        ARMS_SELL_BROWSER_TEXT_WINDOW_INDEX,
        ARMS_SELL_BROWSER_LEFT,
        ARMS_SELL_BROWSER_TOP,
        ARMS_SELL_BROWSER_INITIAL_RIGHT,
        ARMS_SELL_BROWSER_INITIAL_BOTTOM,
    );
    system.emit_byte(TEXT_CTRL_CLEAR_WINDOW);
    system.set_window_rect(
        ARMS_SELL_BROWSER_TEXT_WINDOW_INDEX,
        ARMS_SELL_BROWSER_LEFT,
        ARMS_SELL_BROWSER_TOP,
        ARMS_SELL_BROWSER_FRAME_RIGHT,
        ARMS_SELL_BROWSER_FRAME_BOTTOM,
    );

    for (row, item) in browser
        .visible_items(&state.equipment_stock)
        .into_iter()
        .enumerate()
    {
        let selected = item == Some(browser.selected);
        let line = item.map_or_else(
            || " ".repeat(13),
            |item| arms_sell_browser_row(item, state.equipment_stock[usize::from(item)]),
        );
        system.set_active_cursor(1, (row + 1) as u8);
        if selected {
            system.emit_byte(TEXT_CTRL_INVERSE_TOGGLE);
        }
        for byte in line.bytes().take(13) {
            system.emit_byte(byte);
        }
        if selected {
            system.emit_byte(TEXT_CTRL_INVERSE_TOGGLE);
        }
    }
    system.set_active_window(TALK_SHOP_TEXT_WINDOW_INDEX);
}

pub const fn arms_sell_page_indicator_bytes(
    indicator: crate::shop_runtime::ArmsSellPageIndicator,
) -> Option<[u8; 3]> {
    use crate::shop_runtime::ArmsSellPageIndicator;
    let middle = match indicator {
        ArmsSellPageIndicator::None => return None,
        ArmsSellPageIndicator::Down => ARMS_SELL_BROWSER_PAGE_GLYPH_DOWN,
        ArmsSellPageIndicator::Up => ARMS_SELL_BROWSER_PAGE_GLYPH_UP,
        ArmsSellPageIndicator::Both => ARMS_SELL_BROWSER_PAGE_GLYPH_BOTH,
    };
    Some([
        ARMS_SELL_BROWSER_PAGE_BADGE_OPEN,
        middle,
        ARMS_SELL_BROWSER_PAGE_BADGE_CLOSE,
    ])
}

pub fn arms_sell_browser_row(item: u8, count: u8) -> String {
    let label = crate::EQUIPMENT_SHORT_LABELS
        .get(usize::from(item))
        .copied()
        .unwrap_or("");
    let content = if count == u8::MAX {
        label.to_string()
    } else {
        format!("{count:>2}-{label}")
    };
    format!("{:<13}", truncate_ascii_chars(&content, 13))
}

pub fn active_arms_sell_browser(state: &PlayState) -> Option<crate::shop_runtime::ArmsSellBrowser> {
    use crate::shop_runtime::ArmsShopState;
    use crate::shop_session::ActiveShopSession;

    let shop_state = match state.active_shop.as_ref()? {
        ActiveShopSession::Arms(shop_state)
        | ActiveShopSession::ArmsLocal(shop_state, _)
        | ActiveShopSession::ArmsStocked(shop_state, _) => shop_state,
        _ => return None,
    };
    match *shop_state {
        ArmsShopState::SellPickItem(browser) | ArmsShopState::SellConfirm { browser, .. } => {
            Some(browser)
        }
        _ => None,
    }
}

pub fn active_arms_sell_page_indicator(
    state: &PlayState,
) -> Option<crate::shop_runtime::ArmsSellPageIndicator> {
    active_arms_sell_browser(state).map(|browser| browser.page_indicator(&state.equipment_stock))
}

/// Inclusive absolute screen rows whose paired bracket caps form the
/// active shop-owned side panel. The local row ranges are `1..=7` for
/// the inn register and `1..=4` for the arms sell browser.
pub fn active_shop_side_panel_border_rows(state: &PlayState) -> Option<(u8, u8)> {
    match state.active_shop.as_ref()? {
        crate::shop_session::ActiveShopSession::Innkeeper(
            crate::shop_runtime::InnkeeperState::PickUpCompanion { .. },
        ) => Some((
            INN_PICKUP_REGISTER_BORDER_FIRST_ROW,
            INN_PICKUP_REGISTER_BORDER_LAST_ROW,
        )),
        crate::shop_session::ActiveShopSession::Arms(
            crate::shop_runtime::ArmsShopState::SellPickItem(_)
            | crate::shop_runtime::ArmsShopState::SellConfirm { .. },
        )
        | crate::shop_session::ActiveShopSession::ArmsLocal(
            crate::shop_runtime::ArmsShopState::SellPickItem(_)
            | crate::shop_runtime::ArmsShopState::SellConfirm { .. },
            _,
        )
        | crate::shop_session::ActiveShopSession::ArmsStocked(
            crate::shop_runtime::ArmsShopState::SellPickItem(_)
            | crate::shop_runtime::ArmsShopState::SellConfirm { .. },
            _,
        ) => Some((
            ARMS_SELL_BROWSER_BORDER_FIRST_ROW,
            ARMS_SELL_BROWSER_BORDER_LAST_ROW,
        )),
        _ => None,
    }
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
    //
    // These two rows emit their trailing blanks trimmed. The window was
    // just cleared, so the cells are already blank and the output is
    // identical - but emitting a full fifteen glyphs into a
    // fifteen-column window wraps the cursor onto the next row, and
    // from the window's own last row that wrap scrolls the whole panel
    // up by one. The roster rows above can wrap harmlessly and must not
    // be trimmed, because a highlighted row's inverse video has to
    // cover all fifteen cells.
    let counter_row = STATS_COUNTER_TOP - STATS_PANEL_TEXT_TOP;
    let date_row = STATS_COUNTER_BOTTOM - STATS_PANEL_TEXT_TOP;
    for (row, line) in [
        (counter_row, render_stats_panel_counter_row(state)),
        (date_row, render_stats_panel_date_row(&state.clock)),
    ] {
        system.set_active_cursor(0, row);
        for byte in line.trim_end().bytes().take(STATS_PANEL_WIDTH) {
            system.emit_byte(byte);
        }
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
    system.set_active_window(MESSAGE_TEXT_WINDOW_INDEX);
    let prompt_row = MESSAGE_WINDOW_BOTTOM - MESSAGE_WINDOW_TOP;
    system.clear_active_row(prompt_row);
    // Column 24 carries the two-colour ribbon end-cap sprite, painted
    // by the chrome pass; the echoed text starts one column in.
    system.set_active_cursor(1, prompt_row);
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
/// Screen row 9: the calendar (`stats-panel.md` section 7).
///
/// The published mechanism is not a centring calculation: a line feed
/// to column 24, three fixed spaces, a fourth **only when both month
/// and day are below ten**, then month, hyphen, day, hyphen and the
/// year zero-padded to three digits. `4-5-139` lands in columns
/// 28..=34 centred on 31 and `12-25-139` in 27..=35 also centred, but
/// `12-5-139` sits one cell left of true centre - that is the
/// original's behaviour, not a bug to correct.
fn render_stats_panel_date_row(clock: &GameClock) -> String {
    let leading = if clock.month < 10 && clock.day < 10 {
        4
    } else {
        3
    };
    fixed_panel_line(&format!(
        "{}{}-{}-{:03}",
        " ".repeat(leading),
        clock.month,
        clock.day,
        clock.year
    ))
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

fn fixed_panel_line(value: &str) -> String {
    let mut line = truncate_ascii_chars(value, STATS_PANEL_WIDTH);
    while line.chars().count() < STATS_PANEL_WIDTH {
        line.push(' ');
    }
    line
}

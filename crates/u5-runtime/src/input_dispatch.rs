//! The shell-agnostic input dispatcher: takes a key + suffix, mutates PlayState, returns whether to keep going. Used by both u5-tui (terminal) and u5-bevy (window).

use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayInputDisposition {
    Continue,
    Quit,
}

pub fn handle_play_key_input(
    state: &mut PlayState,
    key: char,
    suffix: &str,
    game_dir: &Path,
) -> io::Result<PlayInputDisposition> {
    if state.endgame.is_some() {
        return Ok(handle_endgame_key_input(state, key, suffix));
    }
    if state.active_blackthorn.is_some() {
        return handle_active_blackthorn_key_input(state, key, suffix, game_dir);
    }
    if state.active_z_stats.is_some() {
        return Ok(handle_active_z_stats_key_input(state, key, suffix));
    }
    if state.active_shop.is_some() {
        return Ok(handle_active_shop_key_input(state, key, suffix));
    }
    if state.active_conversation.is_some() {
        return Ok(handle_active_conversation_key_input(state, key, suffix));
    }
    if state.resolve_town_arrest_prompt(key, game_dir)?.is_some() {
        return Ok(PlayInputDisposition::Continue);
    }
    if state.resolve_moongate_prompt(key, game_dir)?.is_some() {
        return Ok(PlayInputDisposition::Continue);
    }
    if state.resolve_natural_moongate_entry(game_dir)?.is_some() {
        return Ok(PlayInputDisposition::Continue);
    }
    if key == PLAY_IGNORED_INPUT_KEY {
        state.message = match suffix {
            "function" => "Function key ignored.",
            _ => "Input ignored.",
        }
        .to_string();
        return Ok(PlayInputDisposition::Continue);
    }
    if key == PLAY_TYPEAHEAD_TOGGLE_KEY {
        state.toggle_typeahead_buffer();
        return Ok(PlayInputDisposition::Continue);
    }
    if key == 'Z' {
        state.z_stats();
        return Ok(PlayInputDisposition::Continue);
    }
    if state.combat_active
        && combat_has_dispatchable_party_actor(state)
        && (state.pending_combat_actor_slot.is_some() || combat_has_active_non_party_actor(state))
        && key == 'C'
        && !suffix.is_empty()
    {
        return handle_combat_cast_key_input(state, suffix, game_dir);
    }
    if key == 'C' && !suffix.is_empty() {
        let turn_before = state.turn;
        let outcome = state.cast_spell_from_suffix(suffix, game_dir)?;
        state.apply_post_turn_effects_after_outcome(turn_before, game_dir, outcome)?;
        return Ok(PlayInputDisposition::Continue);
    }
    if key == 'M' && !suffix.is_empty() {
        if state
            .meditate_shrine_from_suffix(suffix, game_dir)?
            .is_none()
            && state
                .read_codex_urn_at_current_position(game_dir)?
                .is_none()
        {
            state.mix_reagents_from_suffix(suffix);
        }
        return Ok(PlayInputDisposition::Continue);
    }
    if key == 'N' && !suffix.is_empty() {
        state.new_order_from_suffix(suffix);
        return Ok(PlayInputDisposition::Continue);
    }
    if key == 'R' && !suffix.is_empty() {
        let turn_before = state.turn;
        let outcome = state.ready_equipment_from_suffix(suffix);
        state.apply_post_turn_effects_after_outcome(turn_before, game_dir, outcome)?;
        return Ok(PlayInputDisposition::Continue);
    }
    if matches!(key, 'J' | 'j') {
        let turn_before = state.turn;
        let member_index = if suffix.is_empty() {
            None
        } else {
            parse_inline_party_index(suffix)
        };
        let outcome = state.jimmy_facing_with_game_dir_and_member(Some(game_dir), member_index)?;
        state.apply_post_turn_effects_after_outcome(turn_before, game_dir, outcome)?;
        return Ok(PlayInputDisposition::Continue);
    }
    if matches!(key, 'Y' | 'y') && !suffix.trim().is_empty() {
        let turn_before = state.turn;
        let outcome = state.yell_command(non_empty_yell_word(suffix));
        state.apply_post_turn_effects_after_outcome(turn_before, game_dir, outcome)?;
        return Ok(PlayInputDisposition::Continue);
    }
    if state.combat_active {
        return Ok(handle_combat_key_input(state, key, suffix));
    }
    if key == 'q' {
        return Ok(PlayInputDisposition::Quit);
    }
    if matches!(state.area, Area::Dungeon { .. }) && key == 'Q' {
        return Ok(state.exit_to_dos_prompt(parse_inline_yes_no(suffix)));
    }
    let inline_direction = suffix.chars().find_map(Direction::from_play_key);
    let inline_hours = parse_inline_hours(suffix);
    let inline_drink = parse_inline_yes_no(suffix);
    let inline_party_index = parse_inline_party_index(suffix);
    let inline_use_request = parse_inline_use_request(suffix);
    let inline_talk_keyword = non_empty_talk_keyword(suffix);
    if state.handle_dungeon_key_with_inline(
        key,
        game_dir,
        inline_hours,
        inline_drink,
        inline_party_index,
        inline_use_request,
    )? {
        return Ok(PlayInputDisposition::Continue);
    }
    if matches!(key, 'T' | 't') && inline_talk_keyword.is_some() {
        let turn_before = state.turn;
        let outcome = state.talk_facing_with_game_dir_and_keyword(game_dir, inline_talk_keyword)?;
        state.apply_post_turn_effects_after_outcome(turn_before, game_dir, outcome)?;
        return Ok(PlayInputDisposition::Continue);
    }
    if state.handle_top_down_key_with_inline(
        key,
        game_dir,
        inline_direction,
        inline_hours,
        inline_drink,
        inline_use_request,
    )? {
        return Ok(PlayInputDisposition::Continue);
    }
    if matches!(state.area, Area::Dungeon { .. }) {
        state.advance_visual_tick();
        state.message = "Zzzzzz...".to_string();
        return Ok(PlayInputDisposition::Continue);
    }
    state.message = format!("Unhandled command `{key}`.");
    Ok(PlayInputDisposition::Continue)
}

fn handle_active_z_stats_key_input(
    state: &mut PlayState,
    key: char,
    suffix: &str,
) -> PlayInputDisposition {
    state.step_active_z_stats(key, suffix);
    PlayInputDisposition::Continue
}

fn handle_active_shop_key_input(
    state: &mut PlayState,
    key: char,
    suffix: &str,
) -> PlayInputDisposition {
    use crate::shop_runtime::*;
    use crate::shop_session::ActiveShopSession;

    let Some(mut session) = state.active_shop.take() else {
        return PlayInputDisposition::Continue;
    };
    let ctx = ShopTransactionContext {
        party_gold: state.gold,
        speaker_intelligence: state.party_intelligence.first().copied().unwrap_or(0),
        world_hour: state.clock.hour,
        party_size: state.party.len(),
    };
    let key_byte = key as u8;
    let inline_digit = suffix
        .chars()
        .find(|c| c.is_ascii_digit())
        .and_then(|c| c.to_digit(10).map(|d| d as u8))
        .or_else(|| key.to_digit(10).map(|d| d as u8));
    let yes = matches!(key_byte, b'Y' | b'y') || suffix.chars().any(|c| matches!(c, 'Y' | 'y'));
    let no = matches!(key_byte, b'N' | b'n') || suffix.chars().any(|c| matches!(c, 'N' | 'n'));

    let message = match &mut session {
        ActiveShopSession::Arms(s) => {
            let mut prices = [0u16; crate::EQUIPMENT_COUNT];
            prices.copy_from_slice(&crate::EQUIPMENT_BASE_PRICES);
            let mut stock = state.equipment_stock;
            let outcome = match (*s, yes, no, inline_digit) {
                (ArmsShopState::Greeting, _, _, _) => step_arms_shop(
                    s,
                    ArmsShopInput::Key(key_byte),
                    ctx,
                    &mut state.gold,
                    &mut stock,
                    &prices,
                ),
                (ArmsShopState::BuyPickItem | ArmsShopState::SellPickItem, _, _, Some(d)) => {
                    step_arms_shop(
                        s,
                        ArmsShopInput::Item(d),
                        ctx,
                        &mut state.gold,
                        &mut stock,
                        &prices,
                    )
                }
                (
                    ArmsShopState::BuyConfirm { .. } | ArmsShopState::SellConfirm { .. },
                    true,
                    _,
                    _,
                ) => step_arms_shop(
                    s,
                    ArmsShopInput::Confirm(true),
                    ctx,
                    &mut state.gold,
                    &mut stock,
                    &prices,
                ),
                (
                    ArmsShopState::BuyConfirm { .. } | ArmsShopState::SellConfirm { .. },
                    _,
                    true,
                    _,
                ) => step_arms_shop(
                    s,
                    ArmsShopInput::Confirm(false),
                    ctx,
                    &mut state.gold,
                    &mut stock,
                    &prices,
                ),
                _ => ArmsShopOutcome::InvalidInput,
            };
            state.equipment_stock = stock;
            format_arms_outcome(outcome)
        }
        ActiveShopSession::Healer(s) => {
            let mut members: Vec<HealerPartyMemberView> = state
                .party
                .iter()
                .map(|m| HealerPartyMemberView {
                    status: m.status,
                    hp: m.hp,
                    max_hp: m.max_hp,
                })
                .collect();
            let outcome = match (*s, yes, no, inline_digit) {
                (HealerShopState::Greeting, _, _, _) => step_healer_shop(
                    s,
                    HealerShopInput::Key(key_byte),
                    &mut state.gold,
                    &mut members,
                ),
                (HealerShopState::PickService, _, _, _) => {
                    let service = match key_byte {
                        b'C' | b'c' => Some(HealerService::Cure),
                        b'H' | b'h' => Some(HealerService::Heal),
                        b'R' | b'r' => Some(HealerService::Resurrect),
                        _ => None,
                    };
                    if let Some(svc) = service {
                        step_healer_shop(
                            s,
                            HealerShopInput::Service(svc),
                            &mut state.gold,
                            &mut members,
                        )
                    } else {
                        HealerOutcome::InvalidInput
                    }
                }
                (HealerShopState::PickPartyMember { .. }, _, _, Some(d)) if d >= 1 => {
                    step_healer_shop(
                        s,
                        HealerShopInput::Slot(d - 1),
                        &mut state.gold,
                        &mut members,
                    )
                }
                (HealerShopState::Confirm { .. }, true, _, _) => step_healer_shop(
                    s,
                    HealerShopInput::Confirm(true),
                    &mut state.gold,
                    &mut members,
                ),
                (HealerShopState::Confirm { .. }, _, true, _) => step_healer_shop(
                    s,
                    HealerShopInput::Confirm(false),
                    &mut state.gold,
                    &mut members,
                ),
                _ => HealerOutcome::InvalidInput,
            };
            for (i, view) in members.iter().enumerate() {
                if let Some(m) = state.party.get_mut(i) {
                    m.status = view.status;
                    m.hp = view.hp;
                }
            }
            format_healer_outcome(outcome)
        }
        ActiveShopSession::Innkeeper(s) => {
            let scene_marker = active_inn_scene_marker(state);
            match (*s, yes, no, inline_digit) {
                (InnkeeperState::Greeting { inn }, _, _, _) => match inn_main_action(key_byte) {
                    InnMainAction::Rest => {
                        let adjusted_room_rate = inn_base_room_rate(inn);
                        let total_price =
                            quote_inn_rest(inn, state.party.len(), adjusted_room_rate)
                                .map(|quote| quote.total_price)
                                .unwrap_or(0);
                        *s = InnkeeperState::ConfirmRest {
                            inn,
                            adjusted_room_rate,
                            total_price,
                        };
                        format!(
                            "{} room and board costs {total_price} gold. (Y/N)",
                            inn.display_name()
                        )
                    }
                    InnMainAction::LeaveCompanion => {
                        let deposit = inn_leave_companion_deposit(inn_base_room_rate(inn));
                        *s = InnkeeperState::PickLeaveCompanion { inn, deposit };
                        format!("Leave which companion? Deposit is {deposit} gold. (1-6)")
                    }
                    InnMainAction::PickUpCompanion => {
                        let adjusted_lodging_charge =
                            inn_leave_companion_deposit(inn_base_room_rate(inn));
                        let guests = inn_guest_indices_for_scene(&state.inn_registry, scene_marker);
                        if guests.is_empty() {
                            *s = InnkeeperState::Greeting { inn };
                            "No one here is from thy party!".to_string()
                        } else if guests.len() == 1 {
                            let registry_index = guests[0];
                            let bill = state
                                .inn_registry
                                .get(registry_index)
                                .map(|guest| {
                                    inn_pickup_bill(adjusted_lodging_charge, guest.stay_counter)
                                })
                                .unwrap_or(0);
                            *s = InnkeeperState::ConfirmPickUpCompanion {
                                inn,
                                registry_index,
                                adjusted_lodging_charge,
                                bill,
                            };
                            format!("Pickup bill is {bill} gold. (Y/N)")
                        } else {
                            let mut guest_indices = [0usize; INN_REGISTRY_CAP];
                            for (slot, registry_index) in
                                guests.iter().copied().take(INN_REGISTRY_CAP).enumerate()
                            {
                                guest_indices[slot] = registry_index;
                            }
                            let guest_count = guests.len().min(INN_REGISTRY_CAP) as u8;
                            *s = InnkeeperState::PickUpCompanion {
                                inn,
                                guest_indices,
                                guest_count,
                                adjusted_lodging_charge,
                            };
                            format!(
                                "Guest register has {guest_count} companion(s). Pick 1-{guest_count}."
                            )
                        }
                    }
                    InnMainAction::Exit => {
                        *s = InnkeeperState::Exited;
                        "Farewell.".to_string()
                    }
                    InnMainAction::Discard => {
                        "Rest (R), Leave (L), Pick up (P), or Space.".to_string()
                    }
                },
                (
                    InnkeeperState::ConfirmRest {
                        inn,
                        adjusted_room_rate,
                        ..
                    },
                    true,
                    _,
                    _,
                ) => {
                    let result = state.pay_inn_rest(inn, adjusted_room_rate);
                    *s = InnkeeperState::Greeting { inn };
                    match result {
                        Ok(outcome) => apply_paid_inn_rest(state, outcome.quote.total_price),
                        Err(err) => format_inn_error(err),
                    }
                }
                (InnkeeperState::ConfirmRest { inn, .. }, _, true, _) => {
                    *s = InnkeeperState::Greeting { inn };
                    "As you wish.".to_string()
                }
                (InnkeeperState::PickLeaveCompanion { inn, deposit: _ }, _, true, _) => {
                    *s = InnkeeperState::Greeting { inn };
                    "As you wish.".to_string()
                }
                (InnkeeperState::PickLeaveCompanion { inn, deposit }, _, _, Some(d)) if d >= 1 => {
                    let party_index = usize::from(d - 1);
                    *s = InnkeeperState::ConfirmLeaveCompanion {
                        inn,
                        party_index,
                        deposit,
                    };
                    format!("Leave party member {d} for {deposit} gold? (Y/N)")
                }
                (
                    InnkeeperState::ConfirmLeaveCompanion {
                        inn,
                        party_index,
                        deposit,
                    },
                    true,
                    _,
                    _,
                ) => {
                    let result = state.leave_inn_companion(scene_marker, party_index, deposit);
                    *s = InnkeeperState::Greeting { inn };
                    match result {
                        Ok(outcome) => format!(
                            "Left companion {} at the inn for {} gold.",
                            outcome.party_index + 1,
                            outcome.deposit
                        ),
                        Err(err) => format_inn_error(err),
                    }
                }
                (InnkeeperState::ConfirmLeaveCompanion { inn, .. }, _, true, _) => {
                    *s = InnkeeperState::Greeting { inn };
                    "As you wish.".to_string()
                }
                (
                    InnkeeperState::PickUpCompanion {
                        inn,
                        guest_indices,
                        guest_count,
                        adjusted_lodging_charge,
                    },
                    _,
                    true,
                    _,
                ) => {
                    let _ = (guest_indices, guest_count, adjusted_lodging_charge);
                    *s = InnkeeperState::Greeting { inn };
                    "As you wish.".to_string()
                }
                (
                    InnkeeperState::PickUpCompanion {
                        inn,
                        guest_indices,
                        guest_count,
                        adjusted_lodging_charge,
                    },
                    _,
                    _,
                    Some(d),
                ) if d >= 1 && d <= guest_count => {
                    let registry_index = guest_indices[usize::from(d - 1)];
                    let bill = state
                        .inn_registry
                        .get(registry_index)
                        .map(|guest| inn_pickup_bill(adjusted_lodging_charge, guest.stay_counter))
                        .unwrap_or(0);
                    *s = InnkeeperState::ConfirmPickUpCompanion {
                        inn,
                        registry_index,
                        adjusted_lodging_charge,
                        bill,
                    };
                    format!("Pickup bill is {bill} gold. (Y/N)")
                }
                (
                    InnkeeperState::ConfirmPickUpCompanion {
                        inn,
                        registry_index,
                        adjusted_lodging_charge,
                        ..
                    },
                    true,
                    _,
                    _,
                ) => {
                    let result = state.pickup_inn_guest(
                        scene_marker,
                        registry_index,
                        adjusted_lodging_charge,
                    );
                    *s = InnkeeperState::Greeting { inn };
                    match result {
                        Ok(outcome) if outcome.returned_dead_from_poison => format!(
                            "Picked up companion {} for {} gold. Thy friend has died, by the way.",
                            outcome.party_index + 1,
                            outcome.bill
                        ),
                        Ok(outcome) => format!(
                            "Picked up companion {} for {} gold.",
                            outcome.party_index + 1,
                            outcome.bill
                        ),
                        Err(err) => format_inn_error(err),
                    }
                }
                (InnkeeperState::ConfirmPickUpCompanion { inn, .. }, _, true, _) => {
                    *s = InnkeeperState::Greeting { inn };
                    "As you wish.".to_string()
                }
                (InnkeeperState::Exited, _, _, _) => "Farewell.".to_string(),
                _ => "I do not understand.".to_string(),
            }
        }
        ActiveShopSession::Tavern(s) => {
            let mut food = state.food;
            let outcome = match (*s, yes, no) {
                (TavernState::Greeting, _, _) => {
                    step_tavern(s, TavernInput::Key(key_byte), &mut state.gold, &mut food)
                }
                (TavernState::PickService, _, _) => {
                    let svc = match key_byte {
                        b'M' | b'm' => Some(TavernService::Meal),
                        b'D' | b'd' => Some(TavernService::Drink),
                        b'R' | b'r' => Some(TavernService::Rumour),
                        _ => None,
                    };
                    if let Some(svc) = svc {
                        step_tavern(s, TavernInput::Service(svc), &mut state.gold, &mut food)
                    } else {
                        TavernOutcome::InvalidInput
                    }
                }
                (TavernState::ConfirmMeal { .. }, true, _) => {
                    step_tavern(s, TavernInput::Confirm(true), &mut state.gold, &mut food)
                }
                (TavernState::ConfirmMeal { .. }, _, true) => {
                    step_tavern(s, TavernInput::Confirm(false), &mut state.gold, &mut food)
                }
                _ => TavernOutcome::InvalidInput,
            };
            state.food = food;
            format_tavern_outcome(outcome)
        }
        ActiveShopSession::Reagent(s) => {
            let mut stock = state.reagents;
            let prices = [25u16, 20, 10, 30, 15, 40, 50, 65];
            let outcome = match (*s, yes, no, inline_digit) {
                (ReagentShopState::Greeting, _, _, _) => step_reagent_shop(
                    s,
                    ReagentShopInput::Key(key_byte),
                    &mut state.gold,
                    &mut stock,
                    &prices,
                ),
                (ReagentShopState::PickReagent, _, _, Some(d)) => step_reagent_shop(
                    s,
                    ReagentShopInput::Reagent(d),
                    &mut state.gold,
                    &mut stock,
                    &prices,
                ),
                (ReagentShopState::PickQuantity { .. }, _, _, Some(q)) => step_reagent_shop(
                    s,
                    ReagentShopInput::Quantity(q),
                    &mut state.gold,
                    &mut stock,
                    &prices,
                ),
                (ReagentShopState::Confirm { .. }, true, _, _) => step_reagent_shop(
                    s,
                    ReagentShopInput::Confirm(true),
                    &mut state.gold,
                    &mut stock,
                    &prices,
                ),
                (ReagentShopState::Confirm { .. }, _, true, _) => step_reagent_shop(
                    s,
                    ReagentShopInput::Confirm(false),
                    &mut state.gold,
                    &mut stock,
                    &prices,
                ),
                _ => ReagentShopOutcome::InvalidInput,
            };
            state.reagents = stock;
            format_reagent_outcome(outcome)
        }
        ActiveShopSession::HorseTrader(s) => {
            let mut pending = false;
            let outcome = match (*s, yes, no) {
                (HorseTraderState::Greeting, _, _) => step_horse_trader(
                    s,
                    HorseTraderInput::Key(key_byte),
                    &mut state.gold,
                    &mut pending,
                ),
                (HorseTraderState::ConfirmPurchase { .. }, true, _) => step_horse_trader(
                    s,
                    HorseTraderInput::Confirm(true),
                    &mut state.gold,
                    &mut pending,
                ),
                (HorseTraderState::ConfirmPurchase { .. }, _, true) => step_horse_trader(
                    s,
                    HorseTraderInput::Confirm(false),
                    &mut state.gold,
                    &mut pending,
                ),
                _ => HorseTraderOutcome::InvalidInput,
            };
            format_horse_trader_outcome(outcome, pending)
        }
        ActiveShopSession::ShipBroker(s) => {
            let outcome = if let Some(return_world) = state.return_world.as_mut() {
                let delivery_x = return_world.x;
                let delivery_y = return_world.y;
                match (*s, yes, no) {
                    (ShipBrokerState::Greeting { .. }, _, _) => step_ship_broker(
                        s,
                        ShipBrokerInput::Key(key_byte),
                        &mut state.gold,
                        &mut return_world.pending_vehicle,
                        delivery_x,
                        delivery_y,
                    ),
                    (ShipBrokerState::ConfirmPurchase { .. }, true, _) => step_ship_broker(
                        s,
                        ShipBrokerInput::Confirm(true),
                        &mut state.gold,
                        &mut return_world.pending_vehicle,
                        delivery_x,
                        delivery_y,
                    ),
                    (ShipBrokerState::ConfirmPurchase { .. }, _, true) => step_ship_broker(
                        s,
                        ShipBrokerInput::Confirm(false),
                        &mut state.gold,
                        &mut return_world.pending_vehicle,
                        delivery_x,
                        delivery_y,
                    ),
                    _ => ShipBrokerOutcome::InvalidInput,
                }
            } else {
                ShipBrokerOutcome::InvalidInput
            };
            format_ship_broker_outcome(outcome)
        }
        ActiveShopSession::Guild(s) => {
            let mut gems = state.gems;
            let mut keys = state.keys;
            let mut torches = state.torches;
            let mut sextant = state.special_items[crate::SPECIAL_ITEM_SEXTANT_INDEX] != 0;
            let outcome = match (*s, yes, no, inline_digit) {
                (GuildShopState::Greeting, _, _, _) => step_guild_shop(
                    s,
                    GuildShopInput::Key(key_byte),
                    &mut state.gold,
                    &mut gems,
                    &mut keys,
                    &mut torches,
                    &mut sextant,
                ),
                (GuildShopState::PickItem, _, _, _) => {
                    let item = match key_byte {
                        b'G' | b'g' => Some(GuildItem::Gems),
                        b'K' | b'k' => Some(GuildItem::Keys),
                        b'T' | b't' => Some(GuildItem::Torches),
                        b'X' | b'x' => Some(GuildItem::Sextant),
                        _ => None,
                    };
                    if let Some(item) = item {
                        step_guild_shop(
                            s,
                            GuildShopInput::Item(item),
                            &mut state.gold,
                            &mut gems,
                            &mut keys,
                            &mut torches,
                            &mut sextant,
                        )
                    } else {
                        GuildShopOutcome::InvalidInput
                    }
                }
                (GuildShopState::PickQuantity { .. }, _, _, Some(q)) => step_guild_shop(
                    s,
                    GuildShopInput::Quantity(q),
                    &mut state.gold,
                    &mut gems,
                    &mut keys,
                    &mut torches,
                    &mut sextant,
                ),
                (GuildShopState::ConfirmSextant { .. }, true, _, _) => step_guild_shop(
                    s,
                    GuildShopInput::Confirm(true),
                    &mut state.gold,
                    &mut gems,
                    &mut keys,
                    &mut torches,
                    &mut sextant,
                ),
                (GuildShopState::ConfirmSextant { .. }, _, true, _) => step_guild_shop(
                    s,
                    GuildShopInput::Confirm(false),
                    &mut state.gold,
                    &mut gems,
                    &mut keys,
                    &mut torches,
                    &mut sextant,
                ),
                _ => GuildShopOutcome::InvalidInput,
            };
            state.gems = gems;
            state.keys = keys;
            state.torches = torches;
            if sextant {
                state.special_items[crate::SPECIAL_ITEM_SEXTANT_INDEX] = 1;
            }
            format_guild_outcome(outcome)
        }
    };
    state.message = message;

    if !session.is_exited() {
        state.active_shop = Some(session);
    }
    PlayInputDisposition::Continue
}

fn active_inn_scene_marker(state: &PlayState) -> u8 {
    match state.area {
        Area::Town { scene, .. } => scene.byte,
        _ => 0,
    }
}

fn apply_paid_inn_rest(state: &mut PlayState, cost: u16) -> String {
    const INN_REST_HOURS: u8 = 8;
    state.mark_town_rest_sleepers();
    let mut recovered_hp = 0;
    let mut recovered_mana = 0;
    for _ in 0..INN_REST_HOURS {
        state.advance_turn_with_minutes(MINUTES_PER_HOUR);
        let (hp, mana) = state.apply_rest_recovery_tick();
        recovered_hp += hp;
        recovered_mana += mana;
    }
    let woke = state.wake_town_rest_sleepers();
    format!(
        "Rested {INN_REST_HOURS} hours at the inn for {cost} gold; recovered {recovered_hp} HP and {recovered_mana} MP; woke {woke} asleep member(s)."
    )
}

fn format_inn_error(err: InnError) -> String {
    match err {
        InnError::EmptyParty => "No one is here to lodge.".to_string(),
        InnError::PartyTooSmallToLeave => "Thou must keep at least one companion.".to_string(),
        InnError::PartyFull => "Thy party is already full.".to_string(),
        InnError::InvalidPartyIndex { .. } => "That companion is not in thy party.".to_string(),
        InnError::InvalidGuestIndex { .. } | InnError::GuestNotAtInn { .. } => {
            "No one here is from thy party!".to_string()
        }
        InnError::RegistryFull => "The inn has no more room for guests.".to_string(),
        InnError::BelowMinimumGold { minimum, .. } => {
            format!("Thou needest at least {minimum} gold to lodge here.")
        }
        InnError::InsufficientGold { required, .. } => {
            format!("Thou lackest the {required} gold.")
        }
    }
}

fn format_arms_outcome(outcome: crate::shop_runtime::ArmsShopOutcome) -> String {
    use crate::shop_runtime::ArmsShopOutcome::*;
    match outcome {
        EnteredBuy => "Buy: pick an item number.".to_string(),
        EnteredSell => "Sell: pick an item number.".to_string(),
        Exited => "Farewell.".to_string(),
        QuotedBuyPrice { item, price } => {
            format!("Item {item} costs {price} gold. (Y/N)")
        }
        OfferedSellPrice { item, offer } => {
            format!("I will pay {offer} gold for item {item}. (Y/N)")
        }
        Bought { item, paid } => format!("Bought item {item} for {paid} gold."),
        Sold { item, received } => format!("Sold item {item} for {received} gold."),
        Declined => "As you wish.".to_string(),
        BuyRefusedShortFunds { quoted_price, .. } => {
            format!("Thou lackest the {quoted_price} gold needed.")
        }
        SellRefusedNoStock { item } => format!("Thou hast no item {item} to sell."),
        BuyRefusedCapHit { .. } => "Thou canst carry no more of those.".to_string(),
        InvalidInput => "I do not understand.".to_string(),
    }
}

fn format_healer_outcome(outcome: crate::shop_runtime::HealerOutcome) -> String {
    use crate::shop_runtime::HealerOutcome::*;
    match outcome {
        EnteredServiceMenu => "Cure (C), Heal (H), or Resurrect (R)?".to_string(),
        QuotedCost { cost, .. } => format!("That will cost {cost} gold. Pick a member (1-6)."),
        Served { slot, cost, .. } => format!("Served party member {slot} for {cost} gold."),
        RefusedShortFunds { cost } => format!("Thou lackest the {cost} gold."),
        RefusedNotEligible { .. } => "That treatment is not needed.".to_string(),
        Declined => "As you wish.".to_string(),
        Exited => "Farewell.".to_string(),
        InvalidInput => "I do not understand.".to_string(),
    }
}

fn format_tavern_outcome(outcome: crate::shop_runtime::TavernOutcome) -> String {
    use crate::shop_runtime::TavernOutcome::*;
    match outcome {
        EnteredMenu => "Meal (M), Drink (D), or Rumour (R)?".to_string(),
        QuotedMealCost { cost } => format!("A meal costs {cost} gold. (Y/N)"),
        MealServed { cost, food_added } => {
            format!("Meal served: {cost} gold spent, food +{food_added}.")
        }
        DrinkServed { cost } => format!("Drink served for {cost} gold."),
        RumourTold => "The tavern-keeper shares a rumour.".to_string(),
        RefusedShortFunds { cost } => format!("Thou lackest the {cost} gold."),
        Declined => "As you wish.".to_string(),
        Exited => "Farewell.".to_string(),
        InvalidInput => "I do not understand.".to_string(),
    }
}

fn format_reagent_outcome(outcome: crate::shop_runtime::ReagentShopOutcome) -> String {
    use crate::shop_runtime::ReagentShopOutcome::*;
    match outcome {
        EnteredMenu => "Pick a reagent (0-7).".to_string(),
        QuotedUnit {
            reagent,
            unit_price,
        } => {
            format!("Reagent {reagent}: {unit_price} gold each. How many?")
        }
        QuotedTotal {
            quantity, total, ..
        } => {
            format!("{quantity} units total {total} gold. (Y/N)")
        }
        Bought {
            reagent,
            quantity,
            paid,
        } => {
            format!("Bought {quantity} of reagent {reagent} for {paid} gold.")
        }
        RefusedShortFunds { total } => format!("Thou lackest the {total} gold."),
        Declined => "As you wish.".to_string(),
        Exited => "Farewell.".to_string(),
        InvalidInput => "I do not understand.".to_string(),
    }
}

fn format_horse_trader_outcome(
    outcome: crate::shop_runtime::HorseTraderOutcome,
    pending: bool,
) -> String {
    use crate::shop_runtime::HorseTraderOutcome::*;
    match outcome {
        QuotedPrice { price } => format!("A fine steed costs {price} gold. (Y/N)"),
        Purchased { price } => {
            let _ = pending;
            format!("Sold for {price} gold. Thy horse awaits outside.")
        }
        RefusedShortFunds { price } => format!("Thou lackest the {price} gold."),
        Declined => "As you wish.".to_string(),
        Exited => "Farewell.".to_string(),
        InvalidInput => "I do not understand.".to_string(),
    }
}

fn format_ship_broker_outcome(outcome: crate::shop_runtime::ShipBrokerOutcome) -> String {
    use crate::shop_runtime::ShipBrokerOutcome::*;
    match outcome {
        QuotedPurchase { quote } => {
            let item = match quote.kind {
                crate::shops::ShipwrightPurchaseKind::Frigate => "frigate",
                crate::shops::ShipwrightPurchaseKind::Skiff => "skiff",
            };
            format!("A {item} costs {} gold. (Y/N)", quote.price)
        }
        PurchaseApplied { outcome } => match outcome.status {
            crate::shops::ShipwrightPurchaseStatus::QueuedFrigate => {
                format!(
                    "Frigate purchased for {} gold. Delivery is queued.",
                    outcome.quote.price
                )
            }
            crate::shops::ShipwrightPurchaseStatus::QueuedSkiff => {
                format!(
                    "Skiff purchased for {} gold. Delivery is queued.",
                    outcome.quote.price
                )
            }
            crate::shops::ShipwrightPurchaseStatus::AddedSkiffToPendingFrigate => format!(
                "Skiff purchased for {} gold and added to the pending frigate.",
                outcome.quote.price
            ),
            crate::shops::ShipwrightPurchaseStatus::ExistingDeliveryRefusal => {
                "No dock space is available for another delivery.".to_string()
            }
        },
        RefusedShortFunds { required, .. } => format!("Thou lackest the {required} gold."),
        Declined => "As you wish.".to_string(),
        Exited => "Farewell.".to_string(),
        InvalidInput => "I do not understand.".to_string(),
    }
}

fn format_guild_outcome(outcome: crate::shop_runtime::GuildShopOutcome) -> String {
    use crate::shop_runtime::GuildShopOutcome::*;
    match outcome {
        EnteredMenu => "Gems (G), Keys (K), Torches (T), or Sextant (X)?".to_string(),
        QuotedUnit { unit_price, .. } => {
            format!("That costs {unit_price} gold each. How many?")
        }
        QuotedTotal {
            quantity, total, ..
        } => {
            format!("{quantity} units total {total} gold.")
        }
        Bought { quantity, paid, .. } => format!("Bought {quantity} for {paid} gold."),
        SextantPurchased { price } => format!("Sextant sold for {price} gold."),
        RefusedShortFunds { cost } => format!("Thou lackest the {cost} gold."),
        Declined => "As you wish.".to_string(),
        Exited => "Farewell.".to_string(),
        InvalidInput => "I do not understand.".to_string(),
    }
}

fn handle_active_conversation_key_input(
    state: &mut PlayState,
    key: char,
    suffix: &str,
) -> PlayInputDisposition {
    // The conversation loop accepts free-text keyword lines. When the
    // outer dispatcher hands us a single key plus an optional suffix
    // we treat the concatenation as the typed line. Pressing a bare
    // Enter (key `\r` or `\n`) submits an empty line, which closes
    // the conversation via the Bye shortcut.
    let mut line = String::new();
    if !matches!(key, '\r' | '\n' | ' ') {
        line.push(key);
    }
    line.push_str(suffix);
    let line = line.trim().to_string();
    let (_text, _ended) = state.submit_active_conversation_keyword(&line);
    PlayInputDisposition::Continue
}

fn handle_active_blackthorn_key_input(
    state: &mut PlayState,
    key: char,
    suffix: &str,
    game_dir: &Path,
) -> io::Result<PlayInputDisposition> {
    let mut line = String::new();
    if !matches!(key, '\r' | '\n' | ' ') {
        line.push(key);
    }
    line.push_str(suffix);
    state.submit_blackthorn_audience_answer(&line, game_dir)?;
    Ok(PlayInputDisposition::Continue)
}

fn handle_endgame_key_input(
    state: &mut PlayState,
    key: char,
    suffix: &str,
) -> PlayInputDisposition {
    let answer = parse_inline_yes_no(suffix).or_else(|| match key {
        'Y' | 'y' => Some(true),
        'N' | 'n' => Some(false),
        _ => None,
    });
    if let Some(answer) = answer {
        state.resolve_endgame_confirmation(answer);
    } else if state
        .endgame
        .as_ref()
        .is_some_and(EndgameState::is_terminal)
    {
        state.resolve_endgame_confirmation(false);
    } else {
        state.message = "Endgame confirmation requires Y or N.".to_string();
    }
    PlayInputDisposition::Continue
}

fn combat_has_dispatchable_party_actor(state: &PlayState) -> bool {
    state
        .combat_actors
        .iter()
        .take(COMBAT_PARTY_ACTOR_SLOTS)
        .copied()
        .any(combat_actor_is_active_not_dead)
}

fn combat_has_active_non_party_actor(state: &PlayState) -> bool {
    state
        .combat_actors
        .iter()
        .skip(COMBAT_PARTY_ACTOR_SLOTS)
        .copied()
        .any(combat_actor_is_active_not_dead)
}

fn handle_combat_cast_key_input(
    state: &mut PlayState,
    suffix: &str,
    game_dir: &Path,
) -> io::Result<PlayInputDisposition> {
    state.ensure_pending_combat_player_turn();
    let Some(actor_slot) = state.pending_combat_actor_slot.take() else {
        state.message = "No active combatant.".to_string();
        return Ok(PlayInputDisposition::Continue);
    };

    let quickness_roll = state.combat_quickness_dispatch_roll(actor_slot);
    if resolve_quickness_dispatch_consumed(
        state.active_effect_tag,
        state.active_effect_counter,
        quickness_roll,
    ) {
        let ring_pass = state.apply_visible_combat_magic_ring_pass_to_slot(actor_slot);
        state.message =
            combat_magic_ring_pass_message(ring_pass).unwrap_or_else(|| "Quickness!".to_string());
        state.ensure_pending_combat_player_turn();
        return Ok(PlayInputDisposition::Continue);
    }

    let had_foe = combat_has_active_non_party_actor(state);
    let turn_before = state.turn;
    let cast_suffix = combat_cast_suffix_for_actor(suffix, actor_slot);
    let outcome = state.cast_spell_from_suffix(&cast_suffix, game_dir)?;
    state.apply_post_turn_effects_after_outcome(turn_before, game_dir, outcome)?;
    let cast_message = state.message.clone();
    let ring_pass = state.apply_visible_combat_magic_ring_pass_to_slot(actor_slot);
    state.message = combat_magic_ring_pass_message(ring_pass).unwrap_or(cast_message);

    if state.combat_active && had_foe && !combat_has_active_non_party_actor(state) {
        state.apply_combat_round_loop_exit(CombatRoundLoopExit::LeaveCombat);
    } else if state.combat_active {
        state.ensure_pending_combat_player_turn();
    }

    Ok(PlayInputDisposition::Continue)
}

fn handle_combat_key_input(state: &mut PlayState, key: char, suffix: &str) -> PlayInputDisposition {
    state.ensure_pending_combat_player_turn();
    let Some(actor_slot) = state.pending_combat_actor_slot.take() else {
        state.message = "No active combatant.".to_string();
        return PlayInputDisposition::Continue;
    };
    let input = combat_player_command_input_from_key_suffix(key, suffix);
    let quickness_roll = state.combat_quickness_dispatch_roll(actor_slot);
    let Some(application) =
        state.apply_combat_player_command_with_inputs(actor_slot, input, quickness_roll)
    else {
        state.message = "No active combatant.".to_string();
        return PlayInputDisposition::Continue;
    };
    state.message = combat_magic_ring_pass_message(application.ring_pass)
        .unwrap_or_else(|| combat_player_command_message(&application.action));
    if let CombatRoundLoopControl::Exit(exit) = application.control_after {
        state.apply_combat_round_loop_exit(exit);
    } else if matches!(
        application.action,
        CombatPlayerCommandAction::PromptForAttackDirection
    ) {
        state.pending_combat_actor_slot = Some(actor_slot);
    } else if application.control_after.result_code().is_none() {
        state.ensure_pending_combat_player_turn();
    }
    PlayInputDisposition::Continue
}

fn combat_magic_ring_pass_message(pass: Option<CombatMagicRingPassOutcome>) -> Option<String> {
    pass.and_then(|ring_pass| ring_pass.vanished_ring)
        .map(|ring| format!("{} vanished.", equipment_name(ring as usize)))
}

fn combat_cast_suffix_for_actor(suffix: &str, actor_slot: usize) -> String {
    let caster_digit = char::from_digit((actor_slot + 1) as u32, 10).unwrap_or('1');
    let Some((index, first_non_space)) = suffix
        .char_indices()
        .find(|(_, ch)| !ch.is_ascii_whitespace())
    else {
        return caster_digit.to_string();
    };

    if first_non_space.is_ascii_digit() {
        let next_index = index + first_non_space.len_utf8();
        let mut rewritten = String::with_capacity(suffix.len());
        rewritten.push_str(&suffix[..index]);
        rewritten.push(caster_digit);
        rewritten.push_str(&suffix[next_index..]);
        rewritten
    } else {
        let mut rewritten = String::with_capacity(suffix.len() + 1);
        rewritten.push(caster_digit);
        rewritten.push_str(suffix);
        rewritten
    }
}

fn combat_player_command_input_from_key_suffix(
    key: char,
    suffix: &str,
) -> CombatPlayerCommandInput {
    if key.eq_ignore_ascii_case(&'A') {
        if let Some(direction_code) = suffix
            .chars()
            .find_map(Direction::from_play_key)
            .and_then(combat_direction_code_for_direction)
        {
            return CombatPlayerCommandInput::AttackDirection(direction_code);
        }
        return CombatPlayerCommandInput::Key('A');
    }
    if let Some(direction_code) =
        Direction::from_play_key(key).and_then(combat_direction_code_for_direction)
    {
        return CombatPlayerCommandInput::Direction(direction_code);
    }
    CombatPlayerCommandInput::Key(key.to_ascii_uppercase())
}

fn combat_player_command_message(action: &CombatPlayerCommandAction) -> String {
    match action {
        CombatPlayerCommandAction::QuicknessSkipped => "Quickness!".to_string(),
        CombatPlayerCommandAction::ActivePlayerSelection(_) => {
            "Active player selected.".to_string()
        }
        CombatPlayerCommandAction::Pass(_) => "Pass.".to_string(),
        CombatPlayerCommandAction::PromptForAttackDirection => "Attack-".to_string(),
        CombatPlayerCommandAction::StepOrAttack { outcome, .. } => match outcome {
            CombatStepOrAttackPrimitiveOutcome::InactiveActor => "No active combatant.".to_string(),
            CombatStepOrAttackPrimitiveOutcome::OutOfArena { .. } => "Leaving combat.".to_string(),
            CombatStepOrAttackPrimitiveOutcome::Moved { .. } => "Moved.".to_string(),
            CombatStepOrAttackPrimitiveOutcome::Attack { .. } => "Attack.".to_string(),
            CombatStepOrAttackPrimitiveOutcome::BlockedActor { .. }
            | CombatStepOrAttackPrimitiveOutcome::BlockedWall => "Blocked.".to_string(),
        },
        CombatPlayerCommandAction::InvalidDirection { .. } => "Direction?".to_string(),
        CombatPlayerCommandAction::QuitDefeat => "Combat abandoned.".to_string(),
        CombatPlayerCommandAction::XitCleanup { allowed: true } => "Exit combat.".to_string(),
        CombatPlayerCommandAction::XitCleanup { allowed: false } => "Foes remain.".to_string(),
        CombatPlayerCommandAction::Branch { branch, .. } => format!("{branch:?}."),
    }
}

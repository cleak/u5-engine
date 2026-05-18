//! The shell-agnostic input dispatcher: takes a key + suffix, mutates PlayState, returns whether to keep going. Used by both u5-tui (terminal) and u5-bevy (window).

use std::io;
use std::path::Path;

use crate::shop_runtime::HealerService;
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
        return handle_endgame_key_input(state, key, suffix, game_dir);
    }
    if state.active_blackthorn.is_some() {
        return handle_active_blackthorn_key_input(state, key, suffix, game_dir);
    }
    if state.active_yes_no_prompt.is_some() {
        return handle_active_yes_no_prompt_key_input(state, key, suffix, game_dir);
    }
    if state.active_direction_prompt.is_some() {
        return handle_active_direction_prompt_key_input(state, key, suffix, game_dir);
    }
    if state.active_cast_followup.is_some() {
        return handle_active_cast_followup_key_input(state, key, suffix, game_dir);
    }
    if state.active_cast.is_some() {
        return handle_active_cast_key_input(state, key, suffix, game_dir);
    }
    if state.active_rest.is_some() {
        return handle_active_rest_key_input(state, key, suffix, game_dir);
    }
    if state.active_jimmy.is_some() {
        return handle_active_jimmy_key_input(state, key, suffix, game_dir);
    }
    if state.active_shrine.is_some() {
        return handle_active_shrine_key_input(state, key, suffix, game_dir);
    }
    if state.active_mix.is_some() {
        return Ok(handle_active_mix_key_input(state, key, suffix));
    }
    if state.active_new_order.is_some() {
        return Ok(handle_active_new_order_key_input(state, key, suffix));
    }
    if state.active_yell.is_some() {
        return handle_active_yell_key_input(state, key, suffix, game_dir);
    }
    if state.active_ready.is_some() {
        return Ok(handle_active_ready_key_input(state, key, suffix));
    }
    if state.active_use.is_some() {
        return handle_active_use_key_input(state, key, suffix, game_dir);
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
    if key == PLAY_MUSIC_TOGGLE_KEY {
        state.toggle_music();
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
    if state.combat_active {
        return Ok(handle_combat_key_input(state, key, suffix));
    }
    if key == 'Z' {
        state.z_stats();
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
    if key == 'q' {
        return Ok(PlayInputDisposition::Quit);
    }
    if matches!(state.area, Area::Dungeon { .. }) && key == 'Q' {
        if let Some(confirm) = parse_inline_yes_no(suffix) {
            return Ok(state.exit_to_dos_prompt(Some(confirm)));
        }
        state.start_exit_to_dos_prompt();
        return Ok(PlayInputDisposition::Continue);
    }
    let inline_direction = suffix.chars().find_map(Direction::from_play_key);
    let inline_rest = parse_inline_rest_request(suffix);
    let inline_drink = parse_inline_yes_no(suffix);
    let inline_party_index = parse_inline_party_index(suffix);
    let inline_use_request = parse_inline_use_request(suffix);
    let inline_talk_keyword = non_empty_talk_keyword(suffix);
    if state.handle_dungeon_key_with_inline(
        key,
        game_dir,
        inline_rest,
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
        inline_rest,
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

fn handle_active_ready_key_input(
    state: &mut PlayState,
    key: char,
    suffix: &str,
) -> PlayInputDisposition {
    state.step_active_ready(key, suffix);
    PlayInputDisposition::Continue
}

fn handle_active_cast_key_input(
    state: &mut PlayState,
    key: char,
    suffix: &str,
    game_dir: &Path,
) -> io::Result<PlayInputDisposition> {
    let turn_before = state.turn;
    let Some((outcome, combat)) = state.step_active_cast(key, suffix, game_dir)? else {
        return Ok(PlayInputDisposition::Continue);
    };
    state.apply_post_turn_effects_after_outcome(turn_before, game_dir, outcome)?;
    if let Some((actor_slot, had_foe)) = combat {
        state.pending_combat_actor_slot = None;
        let cast_message = state.message.clone();
        let ring_pass = state.apply_visible_combat_magic_ring_pass_to_slot(actor_slot);
        state.message = combat_magic_ring_pass_message(ring_pass).unwrap_or(cast_message);
        if state.combat_active && had_foe && !combat_has_active_non_party_actor(state) {
            state.apply_combat_round_loop_exit(CombatRoundLoopExit::LeaveCombat);
        } else if state.combat_active {
            advance_combat_round_after_actor_and_append_message(state, actor_slot);
        }
    }
    Ok(PlayInputDisposition::Continue)
}

fn handle_active_cast_followup_key_input(
    state: &mut PlayState,
    key: char,
    suffix: &str,
    game_dir: &Path,
) -> io::Result<PlayInputDisposition> {
    let turn_before = state.turn;
    let Some((outcome, combat)) = state.step_active_cast_followup(key, suffix, game_dir)? else {
        return Ok(PlayInputDisposition::Continue);
    };
    state.apply_post_turn_effects_after_outcome(turn_before, game_dir, outcome)?;
    if let Some((actor_slot, had_foe)) = combat {
        state.pending_combat_actor_slot = None;
        let cast_message = state.message.clone();
        let ring_pass = state.apply_visible_combat_magic_ring_pass_to_slot(actor_slot);
        state.message = combat_magic_ring_pass_message(ring_pass).unwrap_or(cast_message);
        if state.combat_active && had_foe && !combat_has_active_non_party_actor(state) {
            state.apply_combat_round_loop_exit(CombatRoundLoopExit::LeaveCombat);
        } else if state.combat_active {
            advance_combat_round_after_actor_and_append_message(state, actor_slot);
        }
    }
    Ok(PlayInputDisposition::Continue)
}

fn handle_active_mix_key_input(
    state: &mut PlayState,
    key: char,
    suffix: &str,
) -> PlayInputDisposition {
    let _ = state.step_active_mix(key, suffix);
    PlayInputDisposition::Continue
}

fn handle_active_rest_key_input(
    state: &mut PlayState,
    key: char,
    suffix: &str,
    game_dir: &Path,
) -> io::Result<PlayInputDisposition> {
    let turn_before = state.turn;
    if let Some(outcome) = state.step_active_rest(key, suffix, game_dir)? {
        state.apply_post_turn_effects_after_outcome(turn_before, game_dir, outcome)?;
    }
    Ok(PlayInputDisposition::Continue)
}

fn handle_active_jimmy_key_input(
    state: &mut PlayState,
    key: char,
    suffix: &str,
    game_dir: &Path,
) -> io::Result<PlayInputDisposition> {
    let turn_before = state.turn;
    if let Some(outcome) = state.step_active_jimmy(key, suffix, game_dir)? {
        state.apply_post_turn_effects_after_outcome(turn_before, game_dir, outcome)?;
    }
    Ok(PlayInputDisposition::Continue)
}

fn handle_active_shrine_key_input(
    state: &mut PlayState,
    key: char,
    suffix: &str,
    game_dir: &Path,
) -> io::Result<PlayInputDisposition> {
    let turn_before = state.turn;
    if let Some(outcome) = state.step_active_shrine(key, suffix, game_dir)? {
        state.apply_post_turn_effects_after_outcome(turn_before, game_dir, outcome)?;
    }
    Ok(PlayInputDisposition::Continue)
}

fn handle_active_new_order_key_input(
    state: &mut PlayState,
    key: char,
    suffix: &str,
) -> PlayInputDisposition {
    let _ = state.step_active_new_order(key, suffix);
    PlayInputDisposition::Continue
}

fn handle_active_yell_key_input(
    state: &mut PlayState,
    key: char,
    suffix: &str,
    game_dir: &Path,
) -> io::Result<PlayInputDisposition> {
    let turn_before = state.turn;
    if let Some(outcome) = state.step_active_yell(key, suffix) {
        state.apply_post_turn_effects_after_outcome(turn_before, game_dir, outcome)?;
    }
    Ok(PlayInputDisposition::Continue)
}

fn handle_active_direction_prompt_key_input(
    state: &mut PlayState,
    key: char,
    suffix: &str,
    game_dir: &Path,
) -> io::Result<PlayInputDisposition> {
    let turn_before = state.turn;
    let combat_push_actor =
        state
            .active_direction_prompt
            .as_ref()
            .and_then(|session| match session.kind {
                DirectionPromptKind::CombatKlimb { actor_slot } => Some(actor_slot),
                DirectionPromptKind::CombatPush { actor_slot } => Some(actor_slot),
                DirectionPromptKind::CombatSjog { actor_slot, .. } => Some(actor_slot),
                _ => None,
            });
    if let Some(outcome) = state.step_active_direction_prompt(key, suffix, game_dir)? {
        if let Some(actor_slot) = combat_push_actor {
            if matches!(outcome, MoveOutcome::PromptDeclined) {
                state.pending_combat_actor_slot = Some(actor_slot);
            } else if state.combat_active {
                advance_combat_round_after_actor_and_append_message(state, actor_slot);
            }
        } else {
            state.apply_post_turn_effects_after_outcome(turn_before, game_dir, outcome)?;
        }
    }
    Ok(PlayInputDisposition::Continue)
}

fn handle_active_yes_no_prompt_key_input(
    state: &mut PlayState,
    key: char,
    suffix: &str,
    game_dir: &Path,
) -> io::Result<PlayInputDisposition> {
    Ok(state
        .step_active_yes_no_prompt(key, suffix, game_dir)?
        .unwrap_or(PlayInputDisposition::Continue))
}

fn handle_active_use_key_input(
    state: &mut PlayState,
    key: char,
    suffix: &str,
    game_dir: &Path,
) -> io::Result<PlayInputDisposition> {
    state.step_active_use(key, suffix, game_dir)?;
    Ok(PlayInputDisposition::Continue)
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
        living_party_members: state
            .party
            .iter()
            .filter(|member| member.living())
            .count()
            .min(u8::MAX as usize) as u8,
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
            let surcharge = if matches!(outcome, ArmsShopOutcome::Bought { .. }) {
                apply_active_shop_surcharge(state, ACTIVE_SHOP_SURCHARGE_ARMS)
            } else {
                None
            };
            append_active_shop_surcharge(format_arms_outcome(outcome), surcharge)
        }
        ActiveShopSession::Healer(s, healer) => match (*s, yes, no, inline_digit) {
            (HealerShopState::Greeting, _, true, _) => {
                *s = HealerShopState::Exited;
                "Farewell.".to_string()
            }
            (HealerShopState::Greeting, true, _, _) => {
                *s = HealerShopState::PickService;
                "Cure (C), Heal (H), or Resurrect (R)?".to_string()
            }
            (HealerShopState::Greeting, _, _, _)
                if matches!(key_byte, b'H' | b'h' | b'Y' | b'y') =>
            {
                *s = HealerShopState::PickService;
                "Cure (C), Heal (H), or Resurrect (R)?".to_string()
            }
            (HealerShopState::Greeting, _, _, _) => {
                *s = HealerShopState::Exited;
                "Farewell.".to_string()
            }
            (HealerShopState::PickService, _, _, _) => match healer_service_action(key_byte) {
                HealerServiceAction::Treatment(treatment) => {
                    let service = healer_service_for_treatment(treatment);
                    let cost = match healer_treatment_fee(*healer, treatment) {
                        HealerTreatmentFee::Bypass => 0,
                        HealerTreatmentFee::Price(cost) => cost,
                    };
                    *s = HealerShopState::PickPartyMember { service, cost };
                    format!("Who needs {}? (1-6)", treatment.display_name())
                }
                HealerServiceAction::Exit => {
                    *s = HealerShopState::Exited;
                    "Farewell.".to_string()
                }
                HealerServiceAction::Discard => "Cure (C), Heal (H), or Resurrect (R)?".to_string(),
            },
            (HealerShopState::PickPartyMember { service, .. }, _, true, _) => {
                *s = HealerShopState::PickService;
                let treatment = healer_treatment_for_service(service);
                format!("Cancelled {}.", treatment.display_name())
            }
            (HealerShopState::PickPartyMember { service, .. }, _, _, Some(d)) if d >= 1 => {
                let target_index = usize::from(d - 1);
                let treatment = healer_treatment_for_service(service);
                let Some(member) = state.party.get(target_index).copied() else {
                    return {
                        state.active_shop = Some(session);
                        state.message = "I do not understand.".to_string();
                        PlayInputDisposition::Continue
                    };
                };
                if !active_healer_target_accepts(treatment, member) {
                    *s = HealerShopState::PickService;
                    "That treatment is not needed.".to_string()
                } else {
                    match healer_treatment_fee(*healer, treatment) {
                        HealerTreatmentFee::Bypass => {
                            let message = match state.buy_healer_treatment(
                                *healer,
                                treatment,
                                target_index,
                            ) {
                                Ok(outcome) => format_healer_treatment_outcome(outcome),
                                Err(err) => format_healer_treatment_error(err),
                            };
                            *s = HealerShopState::PickService;
                            message
                        }
                        HealerTreatmentFee::Price(cost) => {
                            *s = HealerShopState::Confirm {
                                service,
                                slot: d - 1,
                                cost,
                            };
                            format!("{} costs {cost} gold. (Y/N)", treatment.display_name())
                        }
                    }
                }
            }
            (
                HealerShopState::Confirm {
                    service,
                    slot,
                    cost,
                    ..
                },
                true,
                _,
                _,
            ) => {
                let treatment = healer_treatment_for_service(service);
                let message =
                    match state.buy_healer_treatment(*healer, treatment, usize::from(slot)) {
                        Ok(outcome) => {
                            let surcharge =
                                apply_active_shop_surcharge(state, ACTIVE_SHOP_SURCHARGE_HEALER);
                            append_active_shop_surcharge(
                                format_healer_treatment_outcome(outcome),
                                surcharge,
                            )
                        }
                        Err(crate::shops::HealerTreatmentError::InsufficientGold { .. }) => {
                            format!("Thou lackest the {cost} gold.")
                        }
                        Err(err) => format_healer_treatment_error(err),
                    };
                *s = HealerShopState::PickService;
                message
            }
            (HealerShopState::Confirm { service, .. }, _, true, _) => {
                *s = HealerShopState::PickService;
                let treatment = healer_treatment_for_service(service);
                format!("Declined {}.", treatment.display_name())
            }
            (HealerShopState::Exited, _, _, _) => "Farewell.".to_string(),
            _ => "I do not understand.".to_string(),
        },
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
                        Ok(outcome) => {
                            let message = apply_paid_inn_rest(state, outcome.quote.total_price);
                            let surcharge =
                                apply_active_shop_surcharge(state, ACTIVE_SHOP_SURCHARGE_INN_REST);
                            append_active_shop_surcharge(message, surcharge)
                        }
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
                        Ok(outcome) => {
                            let message = format!(
                                "Left companion {} at the inn for {} gold.",
                                outcome.party_index + 1,
                                outcome.deposit
                            );
                            let surcharge =
                                apply_active_shop_surcharge(state, ACTIVE_SHOP_SURCHARGE_INN_GUEST);
                            append_active_shop_surcharge(message, surcharge)
                        }
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
                        Ok(outcome) if outcome.returned_dead_from_poison => {
                            let message = format!(
                                "Picked up companion {} for {} gold. Thy friend has died, by the way.",
                                outcome.party_index + 1,
                                outcome.bill
                            );
                            let surcharge =
                                apply_active_shop_surcharge(state, ACTIVE_SHOP_SURCHARGE_INN_GUEST);
                            append_active_shop_surcharge(message, surcharge)
                        }
                        Ok(outcome) => {
                            let message = format!(
                                "Picked up companion {} for {} gold.",
                                outcome.party_index + 1,
                                outcome.bill
                            );
                            let surcharge =
                                apply_active_shop_surcharge(state, ACTIVE_SHOP_SURCHARGE_INN_GUEST);
                            append_active_shop_surcharge(message, surcharge)
                        }
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
            let outcome = match (*s, yes, no, inline_digit) {
                (TavernState::Greeting { .. }, _, _, _) => step_tavern(
                    s,
                    TavernInput::Key(key_byte),
                    ctx,
                    &mut state.gold,
                    &mut food,
                ),
                (TavernState::Menu { .. } | TavernState::BlueBoarDrinkList { .. }, _, _, _) => {
                    step_tavern(
                        s,
                        TavernInput::Key(key_byte),
                        ctx,
                        &mut state.gold,
                        &mut food,
                    )
                }
                (TavernState::PickProvisionQuantity { .. }, _, true, _) => {
                    *s = TavernState::Exited;
                    TavernOutcome::Exited
                }
                (TavernState::PickProvisionQuantity { .. }, _, _, Some(quantity)) => step_tavern(
                    s,
                    TavernInput::Quantity(u16::from(quantity)),
                    ctx,
                    &mut state.gold,
                    &mut food,
                ),
                _ => TavernOutcome::InvalidInput,
            };
            state.food = food;
            let surcharge = if matches!(
                outcome,
                TavernOutcome::RoundDrinkServed { .. }
                    | TavernOutcome::BlueBoarDrinkServed { .. }
                    | TavernOutcome::ProvisionsPurchased { paid: 1.., .. }
            ) {
                apply_active_shop_surcharge(state, ACTIVE_SHOP_SURCHARGE_TAVERN)
            } else {
                None
            };
            append_active_shop_surcharge(format_tavern_outcome(outcome), surcharge)
        }
        ActiveShopSession::Sage(s) => {
            let line = active_shop_text_line(key, suffix);
            let outcome = match (*s, yes, no) {
                (SageState::Prompt { .. }, _, _) => {
                    step_sage(s, SageInput::Topic(&line), &mut state.gold)
                }
                (SageState::Confirm { .. }, true, _) => {
                    step_sage(s, SageInput::Confirm(true), &mut state.gold)
                }
                (SageState::Confirm { .. }, _, true) => {
                    step_sage(s, SageInput::Confirm(false), &mut state.gold)
                }
                _ => SageOutcome::InvalidInput,
            };
            let surcharge = if matches!(outcome, SageOutcome::RumourPurchased { .. }) {
                apply_active_shop_surcharge(state, ACTIVE_SHOP_SURCHARGE_SAGE)
            } else {
                None
            };
            append_active_shop_surcharge(format_sage_outcome(outcome), surcharge)
        }
        ActiveShopSession::Reagent(s) => {
            let mut stock = state.reagents;
            let outcome = match (*s, inline_digit) {
                (ReagentShopState::Greeting { .. } | ReagentShopState::PickReagent { .. }, _) => {
                    step_reagent_shop(
                        s,
                        ReagentShopInput::Key(key_byte),
                        &mut state.gold,
                        &mut stock,
                    )
                }
                (ReagentShopState::PickQuantity { .. }, Some(q)) => step_reagent_shop(
                    s,
                    ReagentShopInput::Quantity(q),
                    &mut state.gold,
                    &mut stock,
                ),
                _ => ReagentShopOutcome::InvalidInput,
            };
            state.reagents = stock;
            format_reagent_outcome(outcome)
        }
        ActiveShopSession::HorseTrader(s) => {
            let mut pending = false;
            let outcome = match (*s, yes, no) {
                (HorseTraderState::Greeting { .. }, _, _) => step_horse_trader(
                    s,
                    HorseTraderInput::Key(key_byte),
                    &mut state.gold,
                    &mut pending,
                ),
                (HorseTraderState::ConfirmPurchase { stable, price }, true, _) => {
                    if state.gold < price {
                        *s = HorseTraderState::Greeting { stable };
                        HorseTraderOutcome::RefusedShortFunds { price }
                    } else if let Some((x, y)) = horse_sale_position(state) {
                        match state.buy_horse(stable, x, y) {
                            Ok(_) => {
                                pending = true;
                                *s = HorseTraderState::Exited;
                                HorseTraderOutcome::Purchased { price }
                            }
                            Err(_) => {
                                *s = HorseTraderState::Greeting { stable };
                                HorseTraderOutcome::InvalidInput
                            }
                        }
                    } else {
                        *s = HorseTraderState::Greeting { stable };
                        HorseTraderOutcome::InvalidInput
                    }
                }
                (HorseTraderState::ConfirmPurchase { .. }, _, true) => step_horse_trader(
                    s,
                    HorseTraderInput::Confirm(false),
                    &mut state.gold,
                    &mut pending,
                ),
                _ => HorseTraderOutcome::InvalidInput,
            };
            let surcharge = if matches!(outcome, HorseTraderOutcome::Purchased { .. }) {
                apply_active_shop_surcharge(state, ACTIVE_SHOP_SURCHARGE_HORSE)
            } else {
                None
            };
            append_active_shop_surcharge(format_horse_trader_outcome(outcome, pending), surcharge)
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
            let surcharge = if matches!(
                &outcome,
                ShipBrokerOutcome::PurchaseApplied { outcome }
                    if matches!(
                        outcome.status,
                        crate::shops::ShipwrightPurchaseStatus::QueuedFrigate
                            | crate::shops::ShipwrightPurchaseStatus::QueuedSkiff
                            | crate::shops::ShipwrightPurchaseStatus::AddedSkiffToPendingFrigate
                    )
            ) {
                apply_active_shop_surcharge(state, ACTIVE_SHOP_SURCHARGE_SHIP)
            } else {
                None
            };
            append_active_shop_surcharge(format_ship_broker_outcome(outcome), surcharge)
        }
        ActiveShopSession::Guild(s) => {
            let mut gems = state.gems;
            let mut keys = state.keys;
            let mut torches = state.torches;
            let outcome = match (*s, inline_digit) {
                (GuildShopState::Greeting { .. } | GuildShopState::PickItem { .. }, _) => {
                    step_guild_shop(
                        s,
                        GuildShopInput::Key(key_byte),
                        &mut state.gold,
                        &mut gems,
                        &mut keys,
                        &mut torches,
                    )
                }
                (GuildShopState::PickQuantity { .. }, Some(q)) => step_guild_shop(
                    s,
                    GuildShopInput::Quantity(q),
                    &mut state.gold,
                    &mut gems,
                    &mut keys,
                    &mut torches,
                ),
                _ => GuildShopOutcome::InvalidInput,
            };
            state.gems = gems;
            state.keys = keys;
            state.torches = torches;
            format_guild_outcome(outcome)
        }
    };
    state.message = message;

    if !session.is_exited() {
        state.active_shop = Some(session);
    }
    PlayInputDisposition::Continue
}

fn horse_sale_position(state: &PlayState) -> Option<(usize, usize)> {
    let mut candidates = Vec::new();
    let mut push_unique = |pos: Option<(usize, usize)>| {
        if let Some(pos) = pos {
            if !candidates.contains(&pos) {
                candidates.push(pos);
            }
        }
    };

    push_unique(state.adjacent_position(state.player.facing));
    for direction in [
        Direction::East,
        Direction::South,
        Direction::West,
        Direction::North,
        Direction::SouthEast,
        Direction::SouthWest,
        Direction::NorthEast,
        Direction::NorthWest,
    ] {
        push_unique(state.adjacent_position(direction));
    }

    candidates
        .into_iter()
        .find(|(x, y)| state.player_can_land_on_foot(None, *x, *y).unwrap_or(false))
}

fn active_shop_text_line(key: char, suffix: &str) -> String {
    let mut line = String::new();
    if !matches!(key, '\r' | '\n' | ' ') {
        line.push(key);
    }
    line.push_str(suffix);
    line
}

fn active_inn_scene_marker(state: &PlayState) -> u8 {
    match state.area {
        Area::Town { scene, .. } => scene.byte,
        _ => 0,
    }
}

const ACTIVE_SHOP_SURCHARGE_ARMS: u8 = 0x11;
const ACTIVE_SHOP_SURCHARGE_HEALER: u8 = 0x23;
const ACTIVE_SHOP_SURCHARGE_INN_REST: u8 = 0x35;
const ACTIVE_SHOP_SURCHARGE_INN_GUEST: u8 = 0x47;
const ACTIVE_SHOP_SURCHARGE_TAVERN: u8 = 0x59;
const ACTIVE_SHOP_SURCHARGE_SAGE: u8 = 0x6B;
const ACTIVE_SHOP_SURCHARGE_HORSE: u8 = 0x7D;
const ACTIVE_SHOP_SURCHARGE_SHIP: u8 = 0x8F;
fn active_shop_surcharge_sentinel(state: &PlayState) -> u8 {
    state.shared_town_conversation_sentinel()
}

fn active_shop_surcharge_roll_seed(state: &PlayState, family_salt: u8) -> u8 {
    (state.turn as u8).wrapping_mul(37)
        ^ state.clock.month.wrapping_mul(3)
        ^ state.clock.day.wrapping_mul(5)
        ^ state.clock.hour.wrapping_mul(7)
        ^ state.clock.minute.wrapping_mul(11)
        ^ (state.player.x as u8).wrapping_mul(13)
        ^ (state.player.y as u8).wrapping_mul(17)
        ^ (state.gold as u8).wrapping_mul(19)
        ^ family_salt
}

fn apply_active_shop_surcharge(
    state: &mut PlayState,
    family_salt: u8,
) -> Option<ShopSurchargeOutcome> {
    let sentinel = active_shop_surcharge_sentinel(state);
    let roll_seed = active_shop_surcharge_roll_seed(state, family_salt);
    let outcome = apply_shop_surcharge(&mut state.gold, sentinel, roll_seed);
    outcome.applied.then_some(outcome)
}

fn append_active_shop_surcharge(
    mut message: String,
    surcharge: Option<ShopSurchargeOutcome>,
) -> String {
    if let Some(outcome) = surcharge {
        message.push_str(&format!(" Surcharge {} gold.", outcome.surcharge));
    }
    message
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

fn healer_treatment_for_service(service: HealerService) -> HealerTreatment {
    match service {
        HealerService::Cure => HealerTreatment::Cure,
        HealerService::Heal => HealerTreatment::Heal,
        HealerService::Resurrect => HealerTreatment::Resurrect,
    }
}

fn healer_service_for_treatment(treatment: HealerTreatment) -> HealerService {
    match treatment {
        HealerTreatment::Cure => HealerService::Cure,
        HealerTreatment::Heal => HealerService::Heal,
        HealerTreatment::Resurrect => HealerService::Resurrect,
    }
}

fn active_healer_target_accepts(treatment: HealerTreatment, member: PartyMember) -> bool {
    match treatment {
        HealerTreatment::Cure => member.status == b'P',
        HealerTreatment::Heal => member.living() && member.hp < member.max_hp,
        HealerTreatment::Resurrect => member.status == b'D',
    }
}

fn format_healer_treatment_outcome(outcome: HealerTreatmentOutcome) -> String {
    let slot = outcome.target_index + 1;
    match outcome.quote.treatment {
        HealerTreatment::Cure => format!("Cured party member {slot}."),
        HealerTreatment::Heal => format!(
            "Healed party member {slot} to {}/{}.",
            outcome.hp_after, outcome.max_hp_after
        ),
        HealerTreatment::Resurrect => format!(
            "Resurrected party member {slot} ({}/{}).",
            outcome.hp_after, outcome.max_hp_after
        ),
    }
}

fn format_healer_treatment_error(error: crate::shops::HealerTreatmentError) -> String {
    match error {
        crate::shops::HealerTreatmentError::InsufficientGold { required, .. } => {
            format!("Thou lackest the {required} gold.")
        }
        crate::shops::HealerTreatmentError::InvalidTarget { .. } => {
            "I do not understand.".to_string()
        }
        crate::shops::HealerTreatmentError::Untreatable => {
            "That treatment is not needed.".to_string()
        }
    }
}

fn format_tavern_outcome(outcome: crate::shop_runtime::TavernOutcome) -> String {
    use crate::shop_runtime::TavernOutcome::*;
    match outcome {
        EnteredMenu {
            tavern,
            round_letter,
        } => {
            format!(
                "{}: drink round ({round_letter}), provisions (P), or Space.",
                tavern.display_name()
            )
        }
        RoundDrinkServed { tavern, cost } => {
            format!("{} served a round for {cost} gold.", tavern.display_name())
        }
        PickBlueBoarDrink => "Choose Blue Boar drink A-F.".to_string(),
        BlueBoarDrinkServed { choice, cost } => {
            format!("Blue Boar drink {:?} served for {cost} gold.", choice)
        }
        PickProvisionQuantity { tavern, unit_price } => format!(
            "{} provisions cost {unit_price} gold each. Quantity?",
            tavern.display_name()
        ),
        ProvisionsPurchased {
            tavern,
            requested_quantity,
            purchased_quantity,
            paid,
            food_added,
        } => format!(
            "{} sold {purchased_quantity}/{requested_quantity} provisions for {paid} gold; food +{food_added}.",
            tavern.display_name()
        ),
        Declined => "No provisions purchased.".to_string(),
        RefusedShortFunds { cost } => format!("Thou lackest the {cost} gold."),
        RefusedNoLivingParty => "No one can drink right now.".to_string(),
        RefusedNoNeed => "Thou needest no provisions.".to_string(),
        Exited => "Farewell.".to_string(),
        InvalidInput => "I do not understand.".to_string(),
    }
}

fn format_sage_outcome(outcome: crate::shop_runtime::SageOutcome) -> String {
    use crate::shop_runtime::SageOutcome::*;
    match outcome {
        QuotedRumour { quote } => format!(
            "{} costs {} gold. (Y/N)",
            quote.topic.subject, quote.topic.fee
        ),
        RumourPurchased { rendered, .. } => rendered,
        RefusedShortFunds { required, .. } => format!("Thou lackest the {required} gold."),
        InputTooLong { limit, .. } => format!("Ask in {limit} characters or fewer."),
        NoTopicMatch => "That, I cannot help thee with.".to_string(),
        Declined => "As you wish.".to_string(),
        Exited => "Farewell.".to_string(),
        InvalidInput => "I do not understand.".to_string(),
    }
}

fn format_reagent_outcome(outcome: crate::shop_runtime::ReagentShopOutcome) -> String {
    use crate::shop_runtime::ReagentShopOutcome::*;
    match outcome {
        EnteredMenu { herbalist } => {
            format!(
                "{} offers reagents A-E, or Space.",
                herbalist.display_name()
            )
        }
        QuotedUnit {
            herbalist,
            reagent,
            unit_price,
        } => format!(
            "{} sells {} for {unit_price} gold each. Quantity?",
            herbalist.display_name(),
            reagent.display_name()
        ),
        Bought {
            herbalist,
            reagent,
            quantity,
            paid,
        } => format!(
            "{} sold {quantity} {} for {paid} gold.",
            herbalist.display_name(),
            reagent.display_name()
        ),
        RefusedShortFunds { cost } => format!("Thou lackest the {cost} gold."),
        RefusedStockCap { cap, .. } => format!("Thou canst carry only {cap}."),
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
        EnteredMenu { shop } => format!(
            "{}: Keys (A), Gems (B), Torches (C), or Space.",
            shop.display_name()
        ),
        QuotedUnit {
            shop,
            commodity,
            unit_price,
        } => format!(
            "{} sells {} for {unit_price} gold each. Quantity?",
            shop.display_name(),
            commodity.display_name()
        ),
        Bought {
            shop,
            commodity,
            quantity,
            paid,
        } => format!(
            "{} sold {quantity} {} for {paid} gold.",
            shop.display_name(),
            commodity.display_name()
        ),
        RefusedShortFunds { cost } => format!("Thou lackest the {cost} gold."),
        RefusedStockCap { cap, .. } => format!("Thou canst carry only {cap}."),
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
    let (text, ended) = state.submit_active_conversation_keyword(&line);
    if !ended {
        if let Some(session) = state.active_conversation.as_ref() {
            let prompt = session.prompt_message();
            state.message = if prompt.is_empty() {
                text
            } else if text.is_empty() {
                prompt.to_string()
            } else {
                format!("{text} {prompt}")
            };
        }
    }
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
    game_dir: &Path,
) -> io::Result<PlayInputDisposition> {
    state.ensure_endgame_messages_loaded(game_dir)?;
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
    Ok(PlayInputDisposition::Continue)
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
        advance_combat_round_after_actor_and_append_message(state, actor_slot);
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
        advance_combat_round_after_actor_and_append_message(state, actor_slot);
    }

    Ok(PlayInputDisposition::Continue)
}

fn handle_combat_key_input(state: &mut PlayState, key: char, suffix: &str) -> PlayInputDisposition {
    if key == '\u{1b}' && suffix.is_empty() {
        state.message = "Aborted.".to_string();
        return PlayInputDisposition::Continue;
    }

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
        if key.eq_ignore_ascii_case(&'Z')
            && actor_slot < COMBAT_PARTY_ACTOR_SLOTS
            && state
                .party
                .get(actor_slot)
                .copied()
                .is_some_and(PartyMember::living)
        {
            state.z_stats_for_party(actor_slot);
            state.pending_combat_actor_slot = Some(actor_slot);
            return PlayInputDisposition::Continue;
        }
        state.message = "No active combatant.".to_string();
        return PlayInputDisposition::Continue;
    };
    state.message = combat_player_command_application_message(&application);
    if handle_combat_multistage_command(state, actor_slot, &application.action, suffix) {
        return PlayInputDisposition::Continue;
    }
    if let CombatRoundLoopControl::Exit(exit) = application.control_after {
        state.apply_combat_round_loop_exit(exit);
    } else if matches!(
        application.action,
        CombatPlayerCommandAction::PromptForAttackDirection
    ) {
        state.pending_combat_actor_slot = Some(actor_slot);
    } else if application.control_after.result_code().is_none() {
        advance_combat_round_after_actor_and_append_message(state, actor_slot);
    }
    PlayInputDisposition::Continue
}

fn handle_combat_multistage_command(
    state: &mut PlayState,
    actor_slot: usize,
    action: &CombatPlayerCommandAction,
    suffix: &str,
) -> bool {
    let CombatPlayerCommandAction::Branch {
        branch,
        live_actor_gate,
    } = action
    else {
        return false;
    };
    if matches!(
        live_actor_gate,
        CombatCommandLiveActorGate::RejectedDeadOrMissing
    ) {
        state.message = "No active combatant.".to_string();
        return false;
    }

    match branch {
        CombatCommandBranch::Ready => {
            state.start_combat_ready_equipment(actor_slot);
            state.pending_combat_actor_slot = Some(actor_slot);
            true
        }
        CombatCommandBranch::CastSpell => {
            state.start_combat_cast_spell_prompt(
                actor_slot,
                combat_has_active_non_party_actor(state),
            );
            state.pending_combat_actor_slot = Some(actor_slot);
            true
        }
        CombatCommandBranch::ZStats => {
            state.z_stats_for_party(actor_slot);
            state.pending_combat_actor_slot = Some(actor_slot);
            true
        }
        CombatCommandBranch::Yell => {
            match combat_yell_word_from_suffix(suffix) {
                CombatInlineYell::Prompt => {
                    state.message = yell_prompt_message();
                    state.pending_combat_actor_slot = Some(actor_slot);
                }
                CombatInlineYell::Empty => {
                    state.message = YELL_NOTHING_SAID_MESSAGE.to_string();
                    advance_combat_round_after_actor_and_append_message(state, actor_slot);
                }
                CombatInlineYell::Word(word) => {
                    let word = PlayState::normalize_yell_word(word);
                    state.message = format!("Yelled {word}. Nothing happens.");
                    advance_combat_round_after_actor_and_append_message(state, actor_slot);
                }
            }
            true
        }
        CombatCommandBranch::Push => {
            if let Some(direction) = suffix
                .chars()
                .find_map(Direction::from_play_key)
                .filter(|direction| direction.is_cardinal())
            {
                state.push_combat_actor_direction(actor_slot, direction);
                advance_combat_round_after_actor_and_append_message(state, actor_slot);
            } else {
                state.active_direction_prompt = Some(DirectionPromptSession::new(
                    DirectionPromptKind::CombatPush { actor_slot },
                ));
                state.message = state.render_active_direction_prompt();
            }
            true
        }
        CombatCommandBranch::Klimb => {
            if let Some(intent) = suffix.chars().find_map(combat_klimb_vertical_intent) {
                state.klimb_combat_actor_vertical(actor_slot, intent);
                if state.combat_active {
                    advance_combat_round_after_actor_and_append_message(state, actor_slot);
                }
            } else if let Some(direction) = suffix
                .chars()
                .find_map(Direction::from_play_key)
                .filter(|direction| direction.is_cardinal())
            {
                state.klimb_combat_actor_direction(actor_slot, direction);
                if state.combat_active {
                    advance_combat_round_after_actor_and_append_message(state, actor_slot);
                }
            } else {
                state.active_direction_prompt = Some(DirectionPromptSession::new(
                    DirectionPromptKind::CombatKlimb { actor_slot },
                ));
                state.message = state.render_active_direction_prompt();
            }
            true
        }
        CombatCommandBranch::Get
        | CombatCommandBranch::Jimmy
        | CombatCommandBranch::Open
        | CombatCommandBranch::Search => {
            if let Some(direction) = suffix
                .chars()
                .find_map(Direction::from_play_key)
                .filter(|direction| direction.is_cardinal())
            {
                state.combat_sjog_actor_direction(actor_slot, *branch, direction);
                if state.combat_active {
                    advance_combat_round_after_actor_and_append_message(state, actor_slot);
                }
            } else {
                state.active_direction_prompt = Some(DirectionPromptSession::new(
                    DirectionPromptKind::CombatSjog {
                        actor_slot,
                        branch: *branch,
                    },
                ));
                state.message = state.render_active_direction_prompt();
            }
            true
        }
        _ => false,
    }
}

fn combat_klimb_vertical_intent(ch: char) -> Option<ClimbIntent> {
    match ch {
        '<' => Some(ClimbIntent::Up),
        '>' => Some(ClimbIntent::Down),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CombatInlineYell<'a> {
    Prompt,
    Empty,
    Word(&'a str),
}

fn combat_yell_word_from_suffix(suffix: &str) -> CombatInlineYell<'_> {
    if suffix.is_empty() {
        return CombatInlineYell::Prompt;
    }
    match non_empty_yell_word(suffix) {
        Some(word) => CombatInlineYell::Word(word),
        None => CombatInlineYell::Empty,
    }
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
    if key.is_ascii_uppercase() {
        return CombatPlayerCommandInput::Key(key);
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
            CombatStepOrAttackPrimitiveOutcome::OutOfArena { x, y } => {
                format!("Leaving combat at ({x}, {y}).")
            }
            CombatStepOrAttackPrimitiveOutcome::Moved { commit } => {
                let (x, y) = commit.actor_position_after;
                format!("Moved to ({x}, {y}).")
            }
            CombatStepOrAttackPrimitiveOutcome::Attack { .. } => "Attack.".to_string(),
            CombatStepOrAttackPrimitiveOutcome::BlockedActor { target_slot } => {
                format!("Blocked by combatant in slot {target_slot}.")
            }
            CombatStepOrAttackPrimitiveOutcome::BlockedWall => "Blocked by wall.".to_string(),
        },
        CombatPlayerCommandAction::InvalidDirection { .. } => "Direction?".to_string(),
        CombatPlayerCommandAction::QuitDefeat => "Combat abandoned.".to_string(),
        CombatPlayerCommandAction::XitCleanup { allowed: true } => "Exit combat.".to_string(),
        CombatPlayerCommandAction::XitCleanup { allowed: false } => "Foes remain.".to_string(),
        CombatPlayerCommandAction::Branch { branch, .. } => combat_command_branch_message(*branch),
    }
}

fn combat_command_branch_message(branch: CombatCommandBranch) -> String {
    if let Some(label) = combat_command_branch_published_label(branch) {
        return label.to_string();
    }

    match branch {
        CombatCommandBranch::SceneMessageAbort(verb) => {
            format!("{}-Not here!", combat_scene_abort_verb_prefix(verb))
        }
        CombatCommandBranch::Klimb => "Klimb-What?".to_string(),
        CombatCommandBranch::AbortPrompt => "Aborted.".to_string(),
        CombatCommandBranch::ToggleMusic => "Music toggled.".to_string(),
        CombatCommandBranch::Invalid => "What?".to_string(),
        CombatCommandBranch::Attack
        | CombatCommandBranch::CastSpell
        | CombatCommandBranch::QuitDefeat
        | CombatCommandBranch::Ready
        | CombatCommandBranch::XitCleanup
        | CombatCommandBranch::Yell
        | CombatCommandBranch::ZStats
        | CombatCommandBranch::Pass
        | CombatCommandBranch::Get
        | CombatCommandBranch::Jimmy
        | CombatCommandBranch::Open
        | CombatCommandBranch::Push
        | CombatCommandBranch::Search
        | CombatCommandBranch::DWhatRefusal
        | CombatCommandBranch::WWhatRefusal => format!("{branch:?}."),
    }
}

fn combat_player_command_application_message(
    application: &CombatPlayerCommandApplication,
) -> String {
    if let Some(ring_message) = combat_magic_ring_pass_message(application.ring_pass) {
        return ring_message;
    }
    let CombatPlayerCommandAction::StepOrAttack {
        outcome: CombatStepOrAttackPrimitiveOutcome::Attack { target_slot },
        ..
    } = application.action
    else {
        return combat_player_command_message(&application.action);
    };
    combat_weapon_attack_message(target_slot, application.weapon_attack)
}

fn combat_weapon_attack_message(
    target_slot: usize,
    weapon_attack: Option<CombatWeaponAttackApplication>,
) -> String {
    let Some(weapon_attack) = weapon_attack else {
        return "Attack: no readied weapon.".to_string();
    };
    match weapon_attack.resolution {
        CombatWeaponAttackResolution::OutOfRange {
            target_range,
            range_cap,
        } => {
            format!("Attack missed: target range {target_range} exceeds weapon range {range_cap}.")
        }
        CombatWeaponAttackResolution::NoOrdinaryDamage { route } => {
            format!(
                "Attack used {} with no ordinary damage.",
                combat_attack_route_label(route)
            )
        }
        CombatWeaponAttackResolution::Miss { route, .. } => {
            format!("Attack missed with {}.", combat_attack_route_label(route))
        }
        CombatWeaponAttackResolution::Special { route } => {
            format!(
                "Attack used {} with a special weapon effect.",
                combat_attack_route_label(route)
            )
        }
        CombatWeaponAttackResolution::Hit { raw_damage, route } => {
            combat_weapon_damage_message(target_slot, raw_damage, route, weapon_attack)
        }
    }
}

fn combat_attack_route_label(route: CombatWeaponAttackRangeRoute) -> &'static str {
    match route {
        CombatWeaponAttackRangeRoute::Melee => "melee",
        CombatWeaponAttackRangeRoute::Ranged { .. } => "ranged attack",
    }
}

fn combat_weapon_damage_message(
    target_slot: usize,
    raw_damage: i16,
    route: CombatWeaponAttackRangeRoute,
    weapon_attack: CombatWeaponAttackApplication,
) -> String {
    match weapon_attack.damage_application {
        Some(CombatWeaponDamageApplication::Monster {
            damage,
            credited_experience,
            ..
        }) => {
            let target = combat_monster_target_label(target_slot, damage.class);
            if damage.missed {
                return format!("Attack hit {target}, but did no damage.");
            }
            let mut message = format!(
                "Hit {target} for {} damage with {}.",
                damage.applied_damage,
                combat_attack_route_label(route)
            );
            if damage.killed {
                message.push_str(&format!(" {target} is defeated."));
            }
            if let Some(xp) = credited_experience {
                message.push_str(&format!(" Gained {xp} XP."));
            }
            message
        }
        Some(CombatWeaponDamageApplication::Party {
            target_slot,
            damage,
        }) => {
            if damage.missed {
                return format!(
                    "Attack hit party member {}, but did no damage.",
                    target_slot + 1
                );
            }
            let mut message = format!(
                "Hit party member {} for {} damage with {}.",
                target_slot + 1,
                damage.applied_damage,
                combat_attack_route_label(route)
            );
            if damage.killed {
                message.push_str(" Party member fell.");
            }
            message
        }
        None => format!(
            "Attack hit for raw {raw_damage} with {}, but no damage was applied.",
            combat_attack_route_label(route)
        ),
    }
}

fn combat_monster_target_label(target_slot: usize, class: u8) -> String {
    combat_class_stats(class)
        .map(|stats| stats.name.to_string())
        .unwrap_or_else(|| format!("combatant in slot {target_slot}"))
}

fn append_combat_round_walk_summary_after_message(
    state: &mut PlayState,
    application: Option<CombatRoundWalkApplication>,
) {
    let Some(application) = application else {
        return;
    };
    let Some(summary) = combat_round_walk_summary(state, &application) else {
        return;
    };
    if !state.message.is_empty() {
        state.message.push('\n');
    }
    state.message.push_str(&summary);
}

fn drive_combat_round_walk_and_append_message(state: &mut PlayState) {
    if !state.combat_active || state.pending_combat_actor_slot.is_some() {
        return;
    }

    for _ in 0..COMBAT_ACTOR_SLOTS {
        let start_slot = state.next_combat_actor_slot.min(COMBAT_ACTOR_SLOTS);
        let application = state.apply_combat_round_walk_from_slot(start_slot, 30, false);
        state.next_combat_actor_slot = match application.stop_reason {
            CombatRoundWalkStopReason::EndOfRound => 0,
            CombatRoundWalkStopReason::AwaitingPlayer | CombatRoundWalkStopReason::Exit => {
                application.next_slot
            }
        };
        if application.stop_reason == CombatRoundWalkStopReason::AwaitingPlayer {
            state.pending_combat_actor_slot = ready_player_slot_from_input_round_walk(&application);
        }

        let should_stop = !matches!(
            application.stop_reason,
            CombatRoundWalkStopReason::EndOfRound
        ) || state.pending_combat_actor_slot.is_some();
        let exit = combat_round_walk_exit(&application);
        append_combat_round_walk_summary_after_message(state, Some(application));
        if let Some(exit) = exit {
            state.apply_combat_round_loop_exit(exit);
            break;
        }
        if should_stop {
            break;
        }
    }
}

fn advance_combat_round_after_actor_and_append_message(state: &mut PlayState, actor_slot: usize) {
    state.next_combat_actor_slot = actor_slot.saturating_add(1).min(COMBAT_ACTOR_SLOTS);
    drive_combat_round_walk_and_append_message(state);
}

fn ready_player_slot_from_input_round_walk(
    application: &CombatRoundWalkApplication,
) -> Option<usize> {
    application.applications.iter().rev().find_map(|entry| {
        let CombatActorSlotDispatchApplication::Slot { slot, action, .. } = entry else {
            return None;
        };
        if *slot < COMBAT_PARTY_ACTOR_SLOTS
            && matches!(action, CombatActorDispatchAction::PlayerReady)
        {
            Some(*slot)
        } else {
            None
        }
    })
}

fn combat_round_walk_exit(application: &CombatRoundWalkApplication) -> Option<CombatRoundLoopExit> {
    application
        .applications
        .iter()
        .rev()
        .find_map(|entry| match entry {
            CombatActorSlotDispatchApplication::EndOfRound { control }
            | CombatActorSlotDispatchApplication::Slot {
                control_after: control,
                ..
            } => match control {
                CombatRoundLoopControl::Exit(exit) => Some(*exit),
                CombatRoundLoopControl::ContinueActorWalk
                | CombatRoundLoopControl::StartNextRound => None,
            },
        })
}

fn combat_round_walk_summary(
    state: &PlayState,
    application: &CombatRoundWalkApplication,
) -> Option<String> {
    let mut lines = Vec::new();
    for entry in application.applications.iter() {
        let CombatActorSlotDispatchApplication::Slot { slot, action, .. } = entry else {
            continue;
        };
        if let CombatActorDispatchAction::MonsterAi {
            ai_turn: Some(ai_turn),
        } = action
        {
            lines.extend(combat_ai_turn_summary_lines(state, *slot, ai_turn));
        }
        if lines.len() >= 3 {
            break;
        }
    }
    (!lines.is_empty()).then(|| lines.join("\n"))
}

fn combat_ai_turn_summary_lines(
    state: &PlayState,
    slot: usize,
    ai_turn: &CombatAiTurnApplication,
) -> Vec<String> {
    let mut lines = Vec::new();
    if let Some(special) = &ai_turn.special {
        lines.push(combat_ai_special_message(state, special));
    }
    if let Some(monster_attack) = ai_turn.monster_attack {
        lines.push(combat_monster_attack_message(state, monster_attack));
    } else if let Some(movement) = ai_turn.movement {
        lines.push(combat_ai_movement_message(
            &combat_actor_slot_label(state, slot),
            movement,
        ));
    }
    lines
}

fn combat_ai_special_message(state: &PlayState, special: &CombatAiSpecialApplication) -> String {
    match special {
        CombatAiSpecialApplication::Possess {
            actor_slot,
            target_slot,
            outcome,
            ..
        } => match outcome {
            CombatPossessResistanceOutcome::Blocked => format!(
                "{} tried to possess party member {}, but resistance held.",
                combat_actor_slot_label(state, *actor_slot),
                target_slot + 1
            ),
            CombatPossessResistanceOutcome::Landed { .. } => format!(
                "{} possessed party member {}.",
                combat_actor_slot_label(state, *actor_slot),
                target_slot + 1
            ),
        },
        CombatAiSpecialApplication::Blink {
            actor_slot,
            visibility,
        } => {
            let state_word = match visibility.visibility {
                CombatLinkedVisibility::Hidden => "vanished",
                CombatLinkedVisibility::Visible => "reappeared",
            };
            format!(
                "{} {state_word}.",
                combat_actor_slot_label(state, *actor_slot)
            )
        }
        CombatAiSpecialApplication::SummonDaemon { actor_slot, summon } => format!(
            "{} summoned daemon at ({}, {}).",
            combat_actor_slot_label(state, *actor_slot),
            summon.x,
            summon.y
        ),
    }
}

fn combat_monster_attack_message(
    state: &PlayState,
    attack: CombatMonsterAttackApplication,
) -> String {
    let attacker = combat_actor_slot_label(state, attack.attacker_slot);
    if let Some(poison) = attack.poison_status_outcome {
        match poison {
            CombatPoisonStatusAttackOutcome::PoisonedPartyMember { .. } => {
                return format!(
                    "{attacker} poisoned party member {}.",
                    attack.target_slot + 1
                );
            }
            CombatPoisonStatusAttackOutcome::FallbackDamage { raw_damage } => {
                return format!(
                    "{attacker} poison attack hit party member {} for {raw_damage} raw damage.",
                    attack.target_slot + 1
                );
            }
            CombatPoisonStatusAttackOutcome::NotPoisonStatusClass
            | CombatPoisonStatusAttackOutcome::GateRejected => {}
        }
    }
    match attack.resolution {
        Some(CombatWeaponAttackResolution::OutOfRange {
            target_range,
            range_cap,
        }) => format!(
            "{attacker} attack missed: target range {target_range} exceeds range {range_cap}."
        ),
        Some(CombatWeaponAttackResolution::NoOrdinaryDamage { route }) => format!(
            "{attacker} used {} with no ordinary damage.",
            combat_attack_route_label(route)
        ),
        Some(CombatWeaponAttackResolution::Miss { route, .. }) => format!(
            "{attacker} missed party member {} with {}.",
            attack.target_slot + 1,
            combat_attack_route_label(route)
        ),
        Some(CombatWeaponAttackResolution::Special { route }) => format!(
            "{attacker} used {} with a special effect.",
            combat_attack_route_label(route)
        ),
        Some(CombatWeaponAttackResolution::Hit { route, .. }) => match attack.damage_application {
            Some(CombatWeaponDamageApplication::Party {
                target_slot,
                damage,
            }) => {
                if damage.missed {
                    format!(
                        "{attacker} hit party member {}, but did no damage.",
                        target_slot + 1
                    )
                } else {
                    let mut message = format!(
                        "{attacker} hit party member {} for {} damage with {}.",
                        target_slot + 1,
                        damage.applied_damage,
                        combat_attack_route_label(route)
                    );
                    if damage.killed {
                        message.push_str(" Party member fell.");
                    }
                    message
                }
            }
            Some(CombatWeaponDamageApplication::Monster {
                target_slot,
                damage,
                ..
            }) => {
                let target = combat_monster_target_label(target_slot, damage.class);
                format!(
                    "{attacker} hit {target} for {} damage with {}.",
                    damage.applied_damage,
                    combat_attack_route_label(route)
                )
            }
            None => format!(
                "{attacker} hit with {}, but no damage was applied.",
                combat_attack_route_label(route)
            ),
        },
        None => format!("{attacker} found no attack route."),
    }
}

fn combat_ai_movement_message(actor: &str, movement: CombatAiMovementOutcome) -> String {
    match movement {
        CombatAiMovementOutcome::Teleport { x, y } => {
            format!("{actor} teleported to ({x}, {y}).")
        }
        CombatAiMovementOutcome::Step { x, y, .. } => format!("{actor} moved to ({x}, {y})."),
        CombatAiMovementOutcome::Blocked { surrounded } => {
            if surrounded {
                format!("{actor} is surrounded.")
            } else {
                format!("{actor} is blocked.")
            }
        }
    }
}

fn combat_actor_slot_label(state: &PlayState, slot: usize) -> String {
    if slot < COMBAT_PARTY_ACTOR_SLOTS {
        return format!("Party member {}", slot + 1);
    }
    let class = state
        .combat_actors
        .get(slot)
        .map(|actor| actor.owner_target_class)
        .unwrap_or_default();
    combat_class_stats(class)
        .map(|stats| stats.name.to_string())
        .unwrap_or_else(|| format!("Combatant {slot}"))
}

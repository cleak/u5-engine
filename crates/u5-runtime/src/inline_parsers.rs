//! Inline parsers for command suffixes typed at the play prompt, spell-code helpers, and prompt messages.

use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::*;

pub fn parse_u8_literal(value: &str) -> io::Result<u8> {
    let trimmed = value.trim();
    let parsed = if let Some(hex) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        u8::from_str_radix(hex, 16)
    } else {
        trimmed.parse::<u8>()
    };
    parsed.map_err(|err| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid byte literal `{value}`: {err}"),
        )
    })
}

pub fn parse_i8_literal(value: &str) -> io::Result<i8> {
    value.trim().parse::<i8>().map_err(|err| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid signed byte literal `{value}`: {err}"),
        )
    })
}

pub fn parse_cardinal_direction(value: &str) -> io::Result<Direction> {
    match value.trim().to_ascii_lowercase().as_str() {
        "north" | "n" => Ok(Direction::North),
        "east" | "e" => Ok(Direction::East),
        "south" | "s" => Ok(Direction::South),
        "west" | "w" => Ok(Direction::West),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("direction must be north, east, south, or west, got `{value}`"),
        )),
    }
}

pub fn parse_inline_hours(value: &str) -> Option<u8> {
    let digits: String = value.chars().filter(|ch| ch.is_ascii_digit()).collect();
    if digits.is_empty() {
        None
    } else {
        digits.parse::<u8>().ok()
    }
}

pub fn moonstone_phase_from_inline_number(value: u8) -> Option<usize> {
    (1..=MOONSTONE_SLOT_COUNT as u8)
        .contains(&value)
        .then_some(value as usize - 1)
}

pub fn parse_inline_use_request(value: &str) -> Option<UseItemRequest> {
    let trimmed = value.trim_start();
    if let Some(index) = parse_inline_potion_index(trimmed) {
        return Some(UseItemRequest::Potion {
            index,
            target: parse_inline_target_party_index(trimmed),
        });
    }
    if trimmed
        .get(..3)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("CIM"))
    {
        return Some(UseItemRequest::Scroll {
            index: SCROLL_RESURRECTION_INDEX,
            direction: None,
            target: parse_inline_target_party_index(trimmed),
        });
    }
    if trimmed
        .get(..3)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("CKX"))
    {
        return Some(UseItemRequest::Scroll {
            index: SCROLL_SUMMON_DAEMON_INDEX,
            direction: None,
            target: None,
        });
    }
    if trimmed
        .get(..3)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("IQW"))
    {
        return Some(UseItemRequest::Scroll {
            index: SCROLL_VIEW_INDEX,
            direction: None,
            target: None,
        });
    }
    if trimmed
        .get(..2)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("AM"))
    {
        return Some(UseItemRequest::AmuletOfLordBritish);
    }
    if trimmed
        .get(..2)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("BB"))
    {
        return Some(UseItemRequest::BlackBadge);
    }
    if trimmed
        .get(..2)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("CR"))
    {
        return Some(UseItemRequest::CrownOfLordBritish);
    }
    if trimmed
        .get(..2)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("SC"))
    {
        return Some(UseItemRequest::Sceptre);
    }
    if trimmed
        .get(..2)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("SP"))
    {
        return Some(UseItemRequest::Spyglass);
    }
    if trimmed
        .get(..2)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("AI"))
    {
        return Some(UseItemRequest::Scroll {
            index: SCROLL_NEGATE_MAGIC_INDEX,
            direction: None,
            target: None,
        });
    }
    if trimmed
        .get(..2)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("AT"))
    {
        return Some(UseItemRequest::Scroll {
            index: SCROLL_NEGATE_TIME_INDEX,
            direction: None,
            target: None,
        });
    }
    if trimmed
        .get(..2)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("HR"))
    {
        return Some(UseItemRequest::Scroll {
            index: SCROLL_WIND_CHANGE_INDEX,
            direction: parse_inline_cardinal_direction(trimmed),
            target: None,
        });
    }
    if trimmed
        .get(..2)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("IS"))
    {
        return Some(UseItemRequest::Scroll {
            index: SCROLL_PROTECTION_INDEX,
            direction: None,
            target: None,
        });
    }
    if trimmed
        .get(..2)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("LV"))
    {
        return Some(UseItemRequest::Scroll {
            index: SCROLL_LIGHT_INDEX,
            direction: None,
            target: None,
        });
    }
    let token = trimmed.chars().next()?;
    match token.to_ascii_uppercase() {
        'T' | 'I' => Some(UseItemRequest::Torch),
        'G' | 'V' => Some(UseItemRequest::Gem),
        'B' => Some(UseItemRequest::WoodenBox),
        'C' => Some(UseItemRequest::MagicCarpet),
        'P' => Some(UseItemRequest::HmsCapePlans),
        'K' | 'J' => Some(UseItemRequest::SkullKey),
        'S' => Some(UseItemRequest::Sextant),
        'W' => Some(UseItemRequest::PocketWatch),
        '1'..='8' => token
            .to_digit(10)
            .and_then(|digit| moonstone_phase_from_inline_number(digit as u8))
            .map(UseItemRequest::Moonstone),
        _ => Some(UseItemRequest::Invalid),
    }
}

pub fn parse_inline_potion_index(value: &str) -> Option<usize> {
    const PREFIXES: [(&str, usize); 16] = [
        ("YELLOW", POTION_YELLOW_INDEX),
        ("PURPLE", POTION_PURPLE_INDEX),
        ("ORANGE", POTION_ORANGE_INDEX),
        ("GREEN", POTION_GREEN_INDEX),
        ("WHITE", POTION_WHITE_INDEX),
        ("BLACK", POTION_BLACK_INDEX),
        ("BLUE", POTION_BLUE_INDEX),
        ("RED", POTION_RED_INDEX),
        ("BLA", POTION_BLACK_INDEX),
        ("BLU", POTION_BLUE_INDEX),
        ("YE", POTION_YELLOW_INDEX),
        ("PU", POTION_PURPLE_INDEX),
        ("OR", POTION_ORANGE_INDEX),
        ("GR", POTION_GREEN_INDEX),
        ("WH", POTION_WHITE_INDEX),
        ("RE", POTION_RED_INDEX),
    ];
    PREFIXES.iter().find_map(|(prefix, index)| {
        value
            .get(..prefix.len())
            .is_some_and(|candidate| candidate.eq_ignore_ascii_case(prefix))
            .then_some(*index)
    })
}

pub fn parse_inline_cardinal_direction(value: &str) -> Option<Direction> {
    value.chars().rev().find_map(|ch| match ch {
        '8' => Some(Direction::North),
        '6' => Some(Direction::East),
        '2' => Some(Direction::South),
        '4' => Some(Direction::West),
        _ => None,
    })
}

pub fn parse_inline_yes_no(value: &str) -> Option<bool> {
    value.chars().find_map(|ch| match ch.to_ascii_lowercase() {
        'y' => Some(true),
        'n' => Some(false),
        _ => None,
    })
}

pub fn parse_inline_party_index(value: &str) -> Option<usize> {
    value
        .chars()
        .find_map(|ch| ch.to_digit(10))
        .and_then(|digit| usize::try_from(digit).ok())
        .map(|digit| digit.saturating_sub(1))
}

pub fn parse_inline_target_party_index(value: &str) -> Option<usize> {
    value
        .chars()
        .filter_map(|ch| ch.to_digit(10))
        .nth(1)
        .and_then(|digit| usize::try_from(digit).ok())
        .and_then(|digit| digit.checked_sub(1))
}

pub fn parse_inline_combat_actor_slot(value: &str) -> Option<usize> {
    let mut groups = value
        .split(|ch: char| !ch.is_ascii_digit())
        .filter(|group| !group.is_empty());
    groups.next()?;
    let target = groups.next()?.parse::<usize>().ok()?;
    (1..=COMBAT_ACTOR_SLOTS)
        .contains(&target)
        .then_some(target - 1)
}

pub fn parse_inline_party_swap(value: &str) -> Option<(usize, usize)> {
    let mut digits = value.chars().filter_map(|ch| ch.to_digit(10));
    let first = digits.next()?;
    let second = digits.next()?;
    if first == 0 || second == 0 {
        return None;
    }
    Some(((first - 1) as usize, (second - 1) as usize))
}

pub fn parse_inline_gate_phase_index(value: &str) -> Option<usize> {
    value
        .chars()
        .filter_map(|ch| ch.to_digit(10))
        .nth(1)
        .and_then(|digit| usize::try_from(digit).ok())
        .and_then(|digit| {
            (1..=MOONSTONE_SLOT_COUNT)
                .contains(&digit)
                .then_some(digit - 1)
        })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InlineMixRequest {
    pub spell_index: Option<usize>,
    pub reagent_mask: u8,
    pub amount: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InlineReadyRequest {
    pub party_index: usize,
    pub item_id: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InlineShrineRequest {
    pub mantra: String,
    pub offering: Option<u8>,
}

pub fn parse_inline_ready_request(value: &str) -> io::Result<Option<InlineReadyRequest>> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let parts: Vec<_> = trimmed
        .split(|ch| matches!(ch, '/' | ':' | ','))
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect();
    if parts.len() != 2 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Ready syntax is R<party-slot>/<equipment-id>, for example R1/16.",
        ));
    }
    let party_slot = parse_u8_literal(parts[0]).map_err(|err| {
        io::Error::new(
            err.kind(),
            format!("invalid ready party slot `{}`: {err}", parts[0]),
        )
    })?;
    let item_id = parse_u8_literal(parts[1]).map_err(|err| {
        io::Error::new(
            err.kind(),
            format!("invalid ready equipment id `{}`: {err}", parts[1]),
        )
    })?;
    if party_slot == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Ready party slot must be 1 or greater.",
        ));
    }
    if item_id as usize >= EQUIPMENT_COUNT {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Ready equipment id must be 0..47, got {item_id}."),
        ));
    }
    Ok(Some(InlineReadyRequest {
        party_index: party_slot as usize - 1,
        item_id: item_id as usize,
    }))
}

pub fn parse_inline_mix_request(value: &str) -> io::Result<Option<InlineMixRequest>> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let parts: Vec<_> = trimmed
        .split(|ch| matches!(ch, '/' | ':' | ','))
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect();
    if parts.len() != 3 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Mix syntax is M<spell>/<reagent-mask>/<quantity>, for example MIL/0x80/1.",
        ));
    }
    let spell_code = inline_spell_code(parts[0]);
    if spell_code.is_empty() {
        return Ok(None);
    }
    let reagent_mask = parse_u8_literal(parts[1]).map_err(|err| {
        io::Error::new(
            err.kind(),
            format!("invalid mix reagent mask `{}`: {err}", parts[1]),
        )
    })?;
    let amount = parse_u8_literal(parts[2]).map_err(|err| {
        io::Error::new(
            err.kind(),
            format!("invalid mix quantity `{}`: {err}", parts[2]),
        )
    })?;
    Ok(Some(InlineMixRequest {
        spell_index: spell_index_from_code(&spell_code),
        reagent_mask,
        amount,
    }))
}

pub fn inline_mix_candidate(value: &str) -> bool {
    let parts: Vec<_> = value
        .trim()
        .split(|ch| matches!(ch, '/' | ':' | ','))
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect();
    parts.len() == 3 && !inline_spell_code(parts[0]).is_empty()
}

pub fn parse_inline_shrine_request(value: &str) -> io::Result<Option<InlineShrineRequest>> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let parts: Vec<_> = trimmed
        .split(|ch| matches!(ch, '/' | ':' | ','))
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect();
    if parts.is_empty() {
        return Ok(None);
    }
    if parts.len() > 2 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Shrine syntax is M<mantra> or M<mantra>/<offering-digit>.",
        ));
    }
    let mantra = parts[0].to_string();
    if mantra.is_empty() {
        return Ok(None);
    }
    let offering = if let Some(offering) = parts.get(1) {
        if offering.len() != 1 || !offering.as_bytes()[0].is_ascii_digit() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Shrine offering must be a digit 0 through 9.",
            ));
        }
        Some(offering.as_bytes()[0] - b'0')
    } else {
        None
    };
    Ok(Some(InlineShrineRequest { mantra, offering }))
}

pub fn mix_prompt_message() -> String {
    "Mix what? Use M<spell>/<reagent-mask>/<quantity>, for example MIL/0x80/1.".to_string()
}

pub fn shrine_prompt_message(virtue: ShrineVirtue) -> String {
    format!(
        "Meditate at the Shrine of {}? Use M{} or M{}/<offering-digit>.",
        virtue.name(),
        virtue.mantra(),
        virtue.mantra()
    )
}

pub fn cast_prompt_message() -> String {
    "Cast what? Use C1IL/C1AZ/C1AN2/C1M2/C1AY6/C1MV2/C1CIM2/C1IS/C1RT/C1AI/C1IW/C1IMX/C1AS6/C1LV/C1HR/C1IP6/C1PU/C1DP/C1AG6/C1AEP/C1EIP/C1IQW/C1AWY/C1PRV2/C1AT."
        .to_string()
}

pub fn use_prompt_message() -> String {
    "Use what? Use UT torch, UG gem, UK key, scroll codes, potion colors, USC Sceptre, USP Spyglass, UCR Crown, UAM Amulet, UBB Badge, or U1..U8 Moonstone."
        .to_string()
}

pub fn new_order_prompt_message() -> String {
    "New order? Use N12 to swap party slots 1 and 2.".to_string()
}

pub fn ready_prompt_message() -> String {
    "Ready what? Use R<party-slot>/<equipment-id>, for example R1/16.".to_string()
}

pub fn yell_prompt_message() -> String {
    "Yell what? Use Y<word>.".to_string()
}

pub fn non_empty_yell_word(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then_some(trimmed)
}

pub fn inline_spell_code(value: &str) -> String {
    let mut letters: Vec<_> = value
        .chars()
        .filter(|ch| ch.is_ascii_alphabetic())
        .map(|ch| ch.to_ascii_uppercase())
        .collect();
    letters.sort_unstable();
    letters.into_iter().collect()
}

pub fn spell_index_from_code(code: &str) -> Option<usize> {
    SPELL_CODES.iter().position(|known| *known == code)
}

pub fn spell_scene_bit_for_area(area: Area) -> u8 {
    match area {
        Area::World { .. } => SPELL_SCENE_OVERWORLD,
        Area::Town { .. } => SPELL_SCENE_INDOOR,
        Area::Dungeon { .. } => SPELL_SCENE_DUNGEON,
    }
}

pub fn spell_allowed_in_area(spell_index: usize, area: Area) -> bool {
    SPELL_SCENE_MASKS[spell_index] & spell_scene_bit_for_area(area) != 0
}

pub fn selected_reagent_indices(mask: u8) -> Vec<usize> {
    REAGENT_MASKS
        .iter()
        .copied()
        .enumerate()
        .filter_map(|(index, bit)| (mask & bit != 0).then_some(index))
        .collect()
}

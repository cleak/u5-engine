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
    let token = value.chars().find(|ch| !ch.is_whitespace())?;
    match token.to_ascii_uppercase() {
        'T' | 'I' => Some(UseItemRequest::Torch),
        'G' | 'V' => Some(UseItemRequest::Gem),
        'K' | 'J' => Some(UseItemRequest::Key),
        '1'..='8' => token
            .to_digit(10)
            .and_then(|digit| moonstone_phase_from_inline_number(digit as u8))
            .map(UseItemRequest::Moonstone),
        _ => Some(UseItemRequest::Invalid),
    }
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InlineShrineRequest {
    pub mantra: String,
    pub offering: Option<u8>,
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
    "Cast what? Use C1IL/C1AZ2/C1AN2/C1M2/C1MV2/C1CIM2/C1IS/C1RT/C1AI/C1IW/C1IMX/C1AS/C1LV/C1HR/C1IP6/C1PU/C1DP/C1AG6/C1AEP/C1EIP/C1IQW/C1AWY/C1PRV2/C1AT."
        .to_string()
}

pub fn use_prompt_message() -> String {
    "Use what? Use UT for torch, UG for gem, UK for key, or U1 through U8 for Moonstone phase."
        .to_string()
}

pub fn new_order_prompt_message() -> String {
    "New order? Use N12 to swap party slots 1 and 2.".to_string()
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

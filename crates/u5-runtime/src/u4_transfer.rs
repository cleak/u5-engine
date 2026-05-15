//! Ultima IV transfer helpers for producing a fresh U5 saved game.

use std::fs;
use std::io;
use std::path::Path;

use crate::*;

pub const U5_TRANSFER_MALE_BYTE: u8 = SAVE_GENDER_MALE_BYTE;
pub const U5_TRANSFER_FEMALE_BYTE: u8 = SAVE_GENDER_FEMALE_BYTE;

/// `u4-transfer.md §5` accepted source-side counter ranges. The
/// transfer rejects the entire attempt before writing the
/// destination save when any leading-record value falls outside
/// these bounds.
pub const U4_TRANSFER_GOLD_GEM_FOOD_MAX: u16 = 9999;
pub const U4_TRANSFER_MOVE_MOON_DUNGEON_MAX: u16 = 70;
pub const U4_TRANSFER_CLASS_INDEX_MAX: u8 = 7;

/// `u4-transfer.md §5`: range gate for the `gold`, `gems`, and
/// `food` source-side counters. Returns `true` when the value is
/// inside the accepted `0..=9999` range.
pub const fn u4_transfer_gold_gem_food_in_range(value: u16) -> bool {
    value <= U4_TRANSFER_GOLD_GEM_FOOD_MAX
}

/// `u4-transfer.md §5`: range gate for the `move`, `moon`, and
/// `dungeon` source-side counters. Returns `true` when the value is
/// inside the accepted `0..=70` range.
pub const fn u4_transfer_move_moon_dungeon_in_range(value: u16) -> bool {
    value <= U4_TRANSFER_MOVE_MOON_DUNGEON_MAX
}

/// `u4-transfer.md §5`: range gate for the source-side class index
/// (`0..=7`). Caller falls through to the per-class translation only
/// when this gate accepts.
pub const fn u4_transfer_class_index_in_range(class_index: u8) -> bool {
    class_index <= U4_TRANSFER_CLASS_INDEX_MAX
}

/// `u4-transfer.md §5`: name-byte gate. The transfer accepts only
/// NUL or printable bytes in the imported name field; any other
/// control byte rejects the transfer attempt.
pub const fn u4_transfer_name_byte_accepted(byte: u8) -> bool {
    byte == 0 || (byte >= 0x20 && byte <= 0x7E)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct U4TransferSource {
    pub name: Vec<u8>,
    pub male: bool,
    pub class_index: u8,
    pub strength: u16,
    pub dexterity: u16,
    pub intelligence: u16,
    pub experience: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct U4TransferOverrides {
    pub name: Option<Vec<u8>>,
    pub male: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct U4TransferAvatar {
    pub name: [u8; SAVE_CHARACTER_NAME_LEN],
    pub male: bool,
    pub class_byte: u8,
    pub strength: u8,
    pub dexterity: u8,
    pub intelligence: u8,
    pub experience: u16,
    pub level: u8,
    pub hp: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum U4TransferError {
    InvalidClassIndex(u8),
    InvalidNameByte(u8),
    BlankName,
    SaveTooShort(usize),
}

impl std::fmt::Display for U4TransferError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidClassIndex(index) => {
                write!(f, "U4 transfer class index must be 0..7, got {index}")
            }
            Self::InvalidNameByte(byte) => {
                write!(f, "U4 transfer name contains invalid byte 0x{byte:02x}")
            }
            Self::BlankName => write!(f, "U4 transfer name must not be blank"),
            Self::SaveTooShort(len) => write!(
                f,
                "U5 transfer seed must contain slot 0 record, got {len} bytes"
            ),
        }
    }
}

impl std::error::Error for U4TransferError {}

pub fn u4_transfer_class_byte(class_index: u8) -> Option<u8> {
    match class_index {
        0 => Some(b'M'),
        1 => Some(b'B'),
        2 => Some(b'F'),
        3 => Some(b'D'),
        4 => Some(b'T'),
        5 => Some(b'P'),
        6 => Some(b'R'),
        7 => Some(b'S'),
        _ => None,
    }
}

pub fn u4_transfer_attribute_to_u5(value: u16) -> u8 {
    let converted = match value {
        0..=9 => value,
        10..=29 => ((value - 9) / 2) + 10,
        _ => ((value - 30) / 4) + 20,
    };
    converted.min(u8::MAX as u16) as u8
}

pub fn u4_transfer_strength_to_u5(value: u16) -> u8 {
    u4_transfer_attribute_to_u5(value).max(20)
}

pub fn u4_transfer_experience_to_u5(value: u32) -> u16 {
    (value / 10).min(u16::MAX as u32) as u16
}

pub fn apply_u4_transfer_to_save(
    save: &mut [u8],
    source: &U4TransferSource,
    overrides: Option<&U4TransferOverrides>,
) -> Result<U4TransferAvatar, U4TransferError> {
    if save.len() < SAVE_ROSTER_OFFSET + SAVE_CHARACTER_RECORD_LEN {
        return Err(U4TransferError::SaveTooShort(save.len()));
    }
    let class_byte = u4_transfer_class_byte(source.class_index)
        .ok_or(U4TransferError::InvalidClassIndex(source.class_index))?;
    let name_bytes = overrides
        .and_then(|overrides| overrides.name.as_deref())
        .unwrap_or(&source.name);
    let name = normalize_u4_transfer_name(name_bytes)?;
    let male = overrides
        .and_then(|overrides| overrides.male)
        .unwrap_or(source.male);
    let strength = u4_transfer_strength_to_u5(source.strength);
    let dexterity = u4_transfer_attribute_to_u5(source.dexterity);
    let intelligence = u4_transfer_attribute_to_u5(source.intelligence);
    let experience = u4_transfer_experience_to_u5(source.experience);
    let level = recompute_level_from_experience(experience);
    let hp = u16::from(level) * 30;

    let record = SAVE_ROSTER_OFFSET;
    save[record..record + SAVE_CHARACTER_NAME_LEN].copy_from_slice(&name);
    save[record + SAVE_CHARACTER_GENDER_OFFSET] = if male {
        U5_TRANSFER_MALE_BYTE
    } else {
        U5_TRANSFER_FEMALE_BYTE
    };
    save[record + SAVE_CHARACTER_CLASS_OFFSET] = class_byte;
    save[record + SAVE_CHARACTER_STATUS_OFFSET] = b'G';
    save[record + SAVE_CHARACTER_STR_OFFSET] = strength;
    save[record + SAVE_CHARACTER_DEX_OFFSET] = dexterity;
    save[record + SAVE_CHARACTER_INT_OFFSET] = intelligence;
    save[record + SAVE_CHARACTER_MANA_OFFSET] = intelligence;
    write_u16_at(save, record + SAVE_CHARACTER_HP_OFFSET, hp);
    write_u16_at(save, record + SAVE_CHARACTER_MAX_HP_OFFSET, hp);
    write_u16_at(save, record + SAVE_CHARACTER_EXPERIENCE_OFFSET, experience);
    save[record + SAVE_CHARACTER_LEVEL_OFFSET] = level;

    Ok(U4TransferAvatar {
        name,
        male,
        class_byte,
        strength,
        dexterity,
        intelligence,
        experience,
        level,
        hp,
    })
}

pub fn commit_u4_transfer_save(
    game_dir: &Path,
    source: &U4TransferSource,
    overrides: Option<&U4TransferOverrides>,
) -> io::Result<U4TransferAvatar> {
    let mut save = read_save_image_file(&game_dir.join("BRIT.GAM"), "BRIT.GAM")?;
    let brit_ool = read_brit_ool_plane(game_dir)?;
    let avatar = apply_u4_transfer_to_save(&mut save, source, overrides)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;

    let mut saved_ool = vec![0; SAVED_OOL_LEN];
    saved_ool[OOL_PLANE_LEN..].copy_from_slice(&brit_ool);
    fs::write(game_dir.join("SAVED.OOL"), saved_ool)?;
    fs::write(game_dir.join("SAVED.GAM"), save)?;

    Ok(avatar)
}

fn normalize_u4_transfer_name(
    name_bytes: &[u8],
) -> Result<[u8; SAVE_CHARACTER_NAME_LEN], U4TransferError> {
    let mut name = [0; SAVE_CHARACTER_NAME_LEN];
    let mut copied = 0;
    let mut has_non_space = false;
    for &byte in name_bytes.iter().take(SAVE_CHARACTER_NAME_LEN - 1) {
        if byte == 0 {
            break;
        }
        if !(0x20..=0x7e).contains(&byte) {
            return Err(U4TransferError::InvalidNameByte(byte));
        }
        if byte != b' ' {
            has_non_space = true;
        }
        name[copied] = byte;
        copied += 1;
    }
    for &byte in name_bytes
        .iter()
        .take(SAVE_CHARACTER_NAME_LEN - 1)
        .skip(copied)
    {
        if byte != 0 && !(0x20..=0x7e).contains(&byte) {
            return Err(U4TransferError::InvalidNameByte(byte));
        }
    }
    if !has_non_space {
        return Err(U4TransferError::BlankName);
    }
    Ok(name)
}

fn read_brit_ool_plane(game_dir: &Path) -> io::Result<Vec<u8>> {
    let bytes = fs::read(game_dir.join("BRIT.OOL"))?;
    if bytes.len() != OOL_PLANE_LEN {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "BRIT.OOL must be {OOL_PLANE_LEN} bytes, got {}",
                bytes.len()
            ),
        ));
    }
    Ok(bytes)
}

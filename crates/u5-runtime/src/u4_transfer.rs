//! Ultima IV transfer helpers for producing a fresh U5 saved game.

use std::fs;
use std::io;
use std::path::Path;

use crate::*;

pub const U5_TRANSFER_MALE_BYTE: u8 = SAVE_GENDER_MALE_BYTE;
pub const U5_TRANSFER_FEMALE_BYTE: u8 = SAVE_GENDER_FEMALE_BYTE;

/// `u4-transfer.md §5,§11` published filenames the transfer flow
/// reads and writes. The U5-side transfer seed pair is `BRIT.GAM`
/// (save image) and `BRIT.OOL` (object overlay), the U4-side source
/// is the player disk's `PARTY.SAV`, and the commit destination is
/// the ordinary U5 save pair `SAVED.GAM` / `SAVED.OOL`. The seeds
/// are read-only; only the commit step writes anything to disk.
pub const U4_TRANSFER_U5_SEED_GAM_FILENAME: &str = "BRIT.GAM";
/// `u4-transfer.md §5,§11` U5-side seed object-overlay filename.
/// Anchored to [`crate::BRIT_OOL_FILENAME`] so the transfer seed
/// alias and the canonical filename stay one value.
pub const U4_TRANSFER_U5_SEED_OOL_FILENAME: &str = crate::BRIT_OOL_FILENAME;
pub const U4_TRANSFER_U4_SOURCE_FILENAME: &str = "PARTY.SAV";
pub const U4_PARTY_SAV_PLAYER0_OFFSET: usize = 0x08;
pub const U4_PARTY_SAV_CHARACTER_RECORD_LEN: usize = 39;
pub const U4_PARTY_SAV_CHARACTER_XP_OFFSET: usize = 0x04;
pub const U4_PARTY_SAV_CHARACTER_STR_OFFSET: usize = 0x06;
pub const U4_PARTY_SAV_CHARACTER_DEX_OFFSET: usize = 0x08;
pub const U4_PARTY_SAV_CHARACTER_INT_OFFSET: usize = 0x0A;
pub const U4_PARTY_SAV_CHARACTER_NAME_OFFSET: usize = 0x14;
pub const U4_PARTY_SAV_CHARACTER_NAME_LEN: usize = 16;
pub const U4_PARTY_SAV_CHARACTER_SEX_OFFSET: usize = 0x24;
pub const U4_PARTY_SAV_CHARACTER_CLASS_OFFSET: usize = 0x25;
pub const U4_PARTY_SAV_MALE_BYTE: u8 = 0x0B;
pub const U4_PARTY_SAV_FOOD_OFFSET: usize = 0x140;
pub const U4_PARTY_SAV_GOLD_OFFSET: usize = 0x144;
pub const U4_PARTY_SAV_KARMA_OFFSET: usize = 0x146;
pub const U4_PARTY_SAV_KARMA_LEN: usize = U4_TRANSFER_VIRTUE_STANDING_COUNT * 2;
pub const U4_PARTY_SAV_GEMS_OFFSET: usize = 0x158;
pub const U4_PARTY_SAV_REQUIRED_LEN: usize = U4_PARTY_SAV_GEMS_OFFSET + 2;

/// `u4-transfer.md §5` accepted source-side counter ranges. The
/// transfer rejects the entire attempt before writing the
/// destination save when any leading-record value falls outside
/// these bounds.
/// `u4-transfer.md §5` source-side gold/gems/food counter bound.
/// The accepted range `0..=9999` matches the U5 word-counter
/// cap inventory.md §2 documents — a U4 character at the cap
/// transfers without truncation into the U5 carriers. Anchored
/// to [`crate::PARTY_GOLD_CAP`] so the U4-transfer source bound
/// and the U5 carrier cap stay one value.
pub const U4_TRANSFER_GOLD_GEM_FOOD_MAX: u16 = crate::PARTY_GOLD_CAP;
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
    PartySaveTooShort(usize),
    SourceCounterOutOfRange {
        field: &'static str,
        value: u32,
        max: u32,
    },
    NoTransferableData,
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
            Self::PartySaveTooShort(len) => {
                write!(f, "U4 PARTY.SAV is too short, got {len} bytes")
            }
            Self::SourceCounterOutOfRange { field, value, max } => {
                write!(
                    f,
                    "U4 transfer {field} counter must be 0..{max}, got {value}"
                )
            }
            Self::NoTransferableData => write!(f, "U4 PARTY.SAV has no transferable virtue data"),
        }
    }
}

impl std::error::Error for U4TransferError {}

/// `u4-transfer.md §5` virtue-standing word count tested by the
/// "no transferable data" guard. The transfer reads the eight
/// consecutive virtue/karma standing words for Honesty, Compassion,
/// Valor, Justice, Sacrifice, Honor, Spirituality, and Humility —
/// one word per published virtue. Anchored to
/// [`crate::VIRTUE_COUNT`] so the transfer-guard word count and
/// the published virtue count share one source of truth.
pub const U4_TRANSFER_VIRTUE_STANDING_COUNT: usize = crate::VIRTUE_COUNT;

/// `u4-transfer.md §5`: returns `true` when the transfer guard
/// should present the "no transferable data" branch instead of the
/// normal preview. The guard fires only when every virtue-standing
/// word in the supplied buffer is zero. Any nonzero word allows
/// the normal transfer preview to proceed.
pub fn u4_transfer_no_transferable_data(virtue_standings: &[u16]) -> bool {
    virtue_standings.iter().all(|&word| word == 0)
}

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

/// `u4-transfer.md §7` primary-attribute translator band boundaries.
/// Values `0..LOW_BAND_END` (0..=9) pass through unchanged; values
/// `LOW_BAND_END..MID_BAND_END` (10..=29) halve their excess over
/// `LOW_BAND_BIAS` and add `MID_BAND_BIAS` (= 10); values
/// `MID_BAND_END..` (30+) quarter their excess over `HIGH_BAND_BIAS`
/// and add `HIGH_BAND_BIAS_OUT` (= 20). Promote the band edges,
/// biases, and divisors so the translator's piecewise formula has
/// one named source of truth.
pub const U4_TRANSFER_ATTRIBUTE_LOW_BAND_END: u16 = 10;
pub const U4_TRANSFER_ATTRIBUTE_MID_BAND_END: u16 = 30;
pub const U4_TRANSFER_ATTRIBUTE_LOW_BAND_BIAS: u16 = 9;
pub const U4_TRANSFER_ATTRIBUTE_MID_BAND_DIVISOR: u16 = 2;
pub const U4_TRANSFER_ATTRIBUTE_MID_BAND_BIAS_OUT: u16 = 10;
pub const U4_TRANSFER_ATTRIBUTE_HIGH_BAND_BIAS: u16 = 30;
pub const U4_TRANSFER_ATTRIBUTE_HIGH_BAND_DIVISOR: u16 = 4;
pub const U4_TRANSFER_ATTRIBUTE_HIGH_BAND_BIAS_OUT: u16 = 20;
/// `u4-transfer.md §7` Strength floor: after the band-translator
/// runs, Strength alone is floored to 20. Dexterity and Intelligence
/// are not floored. This floor matches the chargen Strength floor
/// (the questionnaire pass also caps Strength below at 20).
/// Anchored to [`crate::CHARGEN_STR_FLOOR`] so the published "20
/// is the minimum starting Strength" rule has one source of truth
/// for both chargen and U4 transfer.
pub const U4_TRANSFER_STRENGTH_FLOOR: u8 = crate::CHARGEN_STR_FLOOR;
/// `u4-transfer.md §7` experience-translator divisor. Source XP is
/// divided by this value, truncating toward zero.
pub const U4_TRANSFER_EXPERIENCE_DIVISOR: u32 = 10;

pub fn u4_transfer_attribute_to_u5(value: u16) -> u8 {
    let converted = if value < U4_TRANSFER_ATTRIBUTE_LOW_BAND_END {
        value
    } else if value < U4_TRANSFER_ATTRIBUTE_MID_BAND_END {
        (value - U4_TRANSFER_ATTRIBUTE_LOW_BAND_BIAS) / U4_TRANSFER_ATTRIBUTE_MID_BAND_DIVISOR
            + U4_TRANSFER_ATTRIBUTE_MID_BAND_BIAS_OUT
    } else {
        (value - U4_TRANSFER_ATTRIBUTE_HIGH_BAND_BIAS) / U4_TRANSFER_ATTRIBUTE_HIGH_BAND_DIVISOR
            + U4_TRANSFER_ATTRIBUTE_HIGH_BAND_BIAS_OUT
    };
    converted.min(u8::MAX as u16) as u8
}

pub fn u4_transfer_strength_to_u5(value: u16) -> u8 {
    u4_transfer_attribute_to_u5(value).max(U4_TRANSFER_STRENGTH_FLOOR)
}

pub fn u4_transfer_experience_to_u5(value: u32) -> u16 {
    (value / U4_TRANSFER_EXPERIENCE_DIVISOR).min(u16::MAX as u32) as u16
}

pub fn read_u4_transfer_source_from_party_sav(game_dir: &Path) -> io::Result<U4TransferSource> {
    let bytes = fs::read(game_dir.join(U4_TRANSFER_U4_SOURCE_FILENAME))?;
    parse_u4_transfer_source_from_party_sav(&bytes)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
}

/// `u4-transfer.md §5,§7` source reader for the public DOS
/// `PARTY.SAV` layout. It imports only the first U4 character record
/// and validates the party-wide counters/virtue words that gate the
/// transfer preview; campaign state remains owned by the U5 seed.
pub fn parse_u4_transfer_source_from_party_sav(
    bytes: &[u8],
) -> Result<U4TransferSource, U4TransferError> {
    if bytes.len() < U4_PARTY_SAV_REQUIRED_LEN {
        return Err(U4TransferError::PartySaveTooShort(bytes.len()));
    }

    validate_u4_source_counter("food", u32_at(bytes, U4_PARTY_SAV_FOOD_OFFSET))?;
    validate_u4_source_counter("gold", u32::from(u16_at(bytes, U4_PARTY_SAV_GOLD_OFFSET)))?;
    validate_u4_source_counter("gems", u32::from(u16_at(bytes, U4_PARTY_SAV_GEMS_OFFSET)))?;

    let karma_words = (0..U4_TRANSFER_VIRTUE_STANDING_COUNT)
        .map(|index| u16_at(bytes, U4_PARTY_SAV_KARMA_OFFSET + index * 2))
        .collect::<Vec<_>>();
    if u4_transfer_no_transferable_data(&karma_words) {
        return Err(U4TransferError::NoTransferableData);
    }

    let record = U4_PARTY_SAV_PLAYER0_OFFSET;
    let class_index = bytes[record + U4_PARTY_SAV_CHARACTER_CLASS_OFFSET];
    if !u4_transfer_class_index_in_range(class_index) {
        return Err(U4TransferError::InvalidClassIndex(class_index));
    }

    let name = bytes[record + U4_PARTY_SAV_CHARACTER_NAME_OFFSET
        ..record + U4_PARTY_SAV_CHARACTER_NAME_OFFSET + U4_PARTY_SAV_CHARACTER_NAME_LEN]
        .to_vec();
    for &byte in &name {
        if !u4_transfer_name_byte_accepted(byte) {
            return Err(U4TransferError::InvalidNameByte(byte));
        }
    }
    normalize_u4_transfer_name(&name)?;

    Ok(U4TransferSource {
        name,
        male: bytes[record + U4_PARTY_SAV_CHARACTER_SEX_OFFSET] == U4_PARTY_SAV_MALE_BYTE,
        class_index,
        strength: u16_at(bytes, record + U4_PARTY_SAV_CHARACTER_STR_OFFSET),
        dexterity: u16_at(bytes, record + U4_PARTY_SAV_CHARACTER_DEX_OFFSET),
        intelligence: u16_at(bytes, record + U4_PARTY_SAV_CHARACTER_INT_OFFSET),
        experience: u32::from(u16_at(bytes, record + U4_PARTY_SAV_CHARACTER_XP_OFFSET)),
    })
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
    let mut save = read_save_image_file(
        &game_dir.join(U4_TRANSFER_U5_SEED_GAM_FILENAME),
        U4_TRANSFER_U5_SEED_GAM_FILENAME,
    )?;
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

fn validate_u4_source_counter(field: &'static str, value: u32) -> Result<(), U4TransferError> {
    if value <= u32::from(U4_TRANSFER_GOLD_GEM_FOOD_MAX) {
        Ok(())
    } else {
        Err(U4TransferError::SourceCounterOutOfRange {
            field,
            value,
            max: u32::from(U4_TRANSFER_GOLD_GEM_FOOD_MAX),
        })
    }
}

fn u16_at(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}

fn u32_at(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

fn read_brit_ool_plane(game_dir: &Path) -> io::Result<Vec<u8>> {
    let bytes = fs::read(game_dir.join(U4_TRANSFER_U5_SEED_OOL_FILENAME))?;
    if bytes.len() != OOL_PLANE_LEN {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{U4_TRANSFER_U5_SEED_OOL_FILENAME} must be {OOL_PLANE_LEN} bytes, got {}",
                bytes.len()
            ),
        ));
    }
    Ok(bytes)
}
